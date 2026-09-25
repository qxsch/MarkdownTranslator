// Helper services for the Markdown translator.
// Deploy: New-AzResourceGroupDeployment -ResourceGroupName <resource-group> -TemplateFile infra/main.bicep -TemplateParameterFile infra/main.bicepparam

targetScope = 'resourceGroup'

@description('Region for all resources.')
param location string = resourceGroup().location

@description('Short prefix for resource names.')
@maxLength(12)
param namePrefix string = 'mdt'

@description('Allow API-key auth on the AI resource (local testing). Set false for Entra ID / managed identity only.')
param allowApiKeys bool = true

@description('Model deployments. The first entry is the default translation model.')
param modelDeployments array = [
  {
    name: 'translate'
    format: 'OpenAI'
    model: 'gpt-5.5'
    version: '2026-04-24'
    sku: 'DataZoneStandard'
    capacity: 250
  }
]

@description('Object id of a developer (user) to grant data-plane access for Entra ID based local testing. Empty to skip.')
param developerPrincipalId string = ''

@description('Also deploy Azure Container Registry + Container Apps hosting for the API.')
param deployApp bool = false

@description('Container image (e.g. myacr.azurecr.io/mdtranslator:1.0). Required when deployApp is true.')
param containerImage string = ''

@description('Key clients must send in the x-api-key header when calling the hosted API.')
@secure()
param apiAuthKey string = ''

var suffix = uniqueString(resourceGroup().id)
var aiName = toLower('${namePrefix}-ai-${suffix}')
var roles = {
  cognitiveServicesOpenAIUser: '5e0bd9bd-7b93-4f28-af87-19fc36ad61bd'
  cognitiveServicesUser: 'a97b65f3-24c7-4388-baec-2e87135dc908'
  acrPull: '7f951dda-4ed3-4680-a7ca-43fe172d538d'
}

resource identity 'Microsoft.ManagedIdentity/userAssignedIdentities@2023-01-31' = {
  name: '${namePrefix}-id-${suffix}'
  location: location
}

// One Foundry (AIServices) resource serves GPT models and Azure Translator (NMT fallback).
resource ai 'Microsoft.CognitiveServices/accounts@2025-06-01' = {
  name: aiName
  location: location
  kind: 'AIServices'
  sku: {
    name: 'S0'
  }
  identity: {
    type: 'SystemAssigned'
  }
  properties: {
    customSubDomainName: aiName
    publicNetworkAccess: 'Enabled'
    disableLocalAuth: !allowApiKeys
  }
}

@batchSize(1)
resource deployments 'Microsoft.CognitiveServices/accounts/deployments@2025-06-01' = [
  for d in modelDeployments: {
    parent: ai
    name: d.name
    sku: {
      name: d.sku
      capacity: d.capacity
    }
    properties: {
      model: {
        format: d.format
        name: d.model
        version: d.version
      }
      versionUpgradeOption: 'NoAutoUpgrade'
      raiPolicyName: 'Microsoft.DefaultV2'
    }
  }
]

var dataPlaneRoles = [roles.cognitiveServicesOpenAIUser, roles.cognitiveServicesUser]

resource identityRoles 'Microsoft.Authorization/roleAssignments@2022-04-01' = [
  for r in dataPlaneRoles: {
    scope: ai
    name: guid(ai.id, identity.id, r)
    properties: {
      principalId: identity.properties.principalId
      principalType: 'ServicePrincipal'
      roleDefinitionId: subscriptionResourceId('Microsoft.Authorization/roleDefinitions', r)
    }
  }
]

resource developerRoles 'Microsoft.Authorization/roleAssignments@2022-04-01' = [
  for r in (empty(developerPrincipalId) ? [] : dataPlaneRoles): {
    scope: ai
    name: guid(ai.id, developerPrincipalId, r)
    properties: {
      principalId: developerPrincipalId
      principalType: 'User'
      roleDefinitionId: subscriptionResourceId('Microsoft.Authorization/roleDefinitions', r)
    }
  }
]

// ---------------------------------------------------------------- optional hosting

resource logs 'Microsoft.OperationalInsights/workspaces@2023-09-01' = if (deployApp) {
  name: '${namePrefix}-log-${suffix}'
  location: location
  properties: {
    sku: {
      name: 'PerGB2018'
    }
    retentionInDays: 30
  }
}

resource acr 'Microsoft.ContainerRegistry/registries@2023-07-01' = if (deployApp) {
  name: toLower('${namePrefix}acr${suffix}')
  location: location
  sku: {
    name: 'Basic'
  }
  properties: {
    adminUserEnabled: false
  }
}

resource acrPull 'Microsoft.Authorization/roleAssignments@2022-04-01' = if (deployApp) {
  scope: acr
  name: guid(acr.id, identity.id, roles.acrPull)
  properties: {
    principalId: identity.properties.principalId
    principalType: 'ServicePrincipal'
    roleDefinitionId: subscriptionResourceId('Microsoft.Authorization/roleDefinitions', roles.acrPull)
  }
}

resource env 'Microsoft.App/managedEnvironments@2024-03-01' = if (deployApp) {
  name: '${namePrefix}-env-${suffix}'
  location: location
  properties: {
    appLogsConfiguration: {
      destination: 'log-analytics'
      logAnalyticsConfiguration: {
        customerId: logs.properties.customerId
        sharedKey: logs.listKeys().primarySharedKey
      }
    }
  }
}

resource app 'Microsoft.App/containerApps@2024-03-01' = if (deployApp && !empty(containerImage)) {
  name: '${namePrefix}-api'
  location: location
  identity: {
    type: 'UserAssigned'
    userAssignedIdentities: {
      '${identity.id}': {}
    }
  }
  properties: {
    managedEnvironmentId: env.id
    configuration: {
      ingress: {
        external: true
        targetPort: 8080
        transport: 'http'
      }
      registries: [
        {
          server: acr.properties.loginServer
          identity: identity.id
        }
      ]
      secrets: [
        {
          name: 'api-auth-key'
          value: apiAuthKey
        }
      ]
    }
    template: {
      containers: [
        {
          name: 'mdtranslator'
          image: containerImage
          resources: {
            cpu: json('2.0')
            memory: '4Gi'
          }
          env: [
            { name: 'AZURE_OPENAI_ENDPOINT', value: 'https://${aiName}.openai.azure.com' }
            { name: 'AZURE_TRANSLATOR_ENDPOINT', value: 'https://${aiName}.cognitiveservices.azure.com' }
            { name: 'AZURE_TRANSLATOR_REGION', value: location }
            { name: 'AZURE_CLIENT_ID', value: identity.properties.clientId }
            { name: 'MDT_TRANSLATE_DEPLOYMENT', value: modelDeployments[0].name }
            { name: 'MDT_API_KEY', secretRef: 'api-auth-key' }
          ]
          probes: [
            {
              type: 'Liveness'
              httpGet: {
                path: '/healthz'
                port: 8080
              }
            }
          ]
        }
      ]
      scale: {
        minReplicas: 0
        maxReplicas: 3
      }
    }
  }
  dependsOn: [
    acrPull
    identityRoles
  ]
}

output aiAccountName string = ai.name
output openAiEndpoint string = 'https://${aiName}.openai.azure.com'
output translatorEndpoint string = 'https://${aiName}.cognitiveservices.azure.com'
output translatorRegion string = location
output managedIdentityClientId string = identity.properties.clientId
output deploymentNames array = [for d in modelDeployments: d.name]
output acrLoginServer string = deployApp ? acr.properties.loginServer : ''
output apiUrl string = (deployApp && !empty(containerImage)) ? 'https://${app.properties.configuration.ingress.fqdn}' : ''
