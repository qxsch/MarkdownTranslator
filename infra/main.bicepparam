using 'main.bicep'

param namePrefix = 'mdt'
param allowApiKeys = true

// translate: primary model (EU data zone). translate-alt is a second candidate model and cross-model judge for the quality evaluation.
param modelDeployments = [
  {
    name: 'translate'
    format: 'OpenAI'
    model: 'gpt-5.5'
    version: '2026-04-24'
    sku: 'DataZoneStandard'
    capacity: 250
  }
  {
    name: 'translate-alt'
    format: 'OpenAI'
    model: 'gpt-5.6-sol'
    version: '2026-07-09'
    sku: 'DataZoneStandard'
    capacity: 250
  }
]

param deployApp = false
