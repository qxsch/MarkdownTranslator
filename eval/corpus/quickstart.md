---
title: Quickstart - Deploy a containerized web API to Azure Container Apps
description: Build a container image, push it to Azure Container Registry and run it on Azure Container Apps with a managed identity.
---

# Quickstart: Deploy a containerized web API

In this quickstart, you build a container image for a small web API, push the image to Azure Container Registry and run it on Azure Container Apps. The app authenticates to other Azure services with a managed identity, so no secrets are stored in code or configuration.

> [!IMPORTANT]
> The resources you create in this quickstart incur costs. When you no longer need them, delete the resource group as described in [Clean up resources](#clean-up-resources).

## Prerequisites

- An Azure subscription. If you don't have one, create a free account before you begin.
- The Az PowerShell module 12.0 or later. Run `Get-Module -ListAvailable Az` to check your version.
- Docker Desktop, running and signed in.
- A clone of the sample repository, which contains the `Dockerfile` and the `src/` folder.

## Build the image

Open a terminal in the root folder of the repository and build the image. The build copies the compiled output into a minimal runtime image, which reduces the attack surface and speeds up cold starts.

```powershell
# Build the image and tag it with the registry name
docker build -t contosoacr.azurecr.io/orders-api:1.0 .

# Sign in to the registry with your Microsoft Entra identity
Connect-AzContainerRegistry -Name contosoacr
```

If the build fails with a permissions error, make sure that Docker Desktop is running and that your user belongs to the `docker-users` group.

## Configure the app

The API reads its settings from environment variables. The following table lists the variables that you must set before you deploy.

| Variable | Required | Description |
|----------|----------|-------------|
| `ORDERS_DB_ENDPOINT` | Yes | Endpoint of the Azure Cosmos DB account that stores the orders. |
| `AZURE_CLIENT_ID` | Yes | Client ID of the user-assigned managed identity. |
| `LOG_LEVEL` | No | Minimum severity that is written to the log. Defaults to `info`. |

```typescript
// Resolve the credential once at startup; it is reused for every request.
const credential = new ManagedIdentityCredential({ clientId: process.env.AZURE_CLIENT_ID });

/* The client caches tokens and refreshes them
   automatically shortly before they expire. */
const client = new CosmosClient({ endpoint, aadCredentials: credential });
```

## Deploy the container app

Run the following command to create the container app. It can take a few minutes until the first revision is ready and receives traffic.

```bash
# Create the app with external ingress on port 8080
az containerapp create --name orders-api --resource-group rg-orders \
  --image contosoacr.azurecr.io/orders-api:1.0 --ingress external --target-port 8080
```

After the deployment has finished, open the URL shown in the output. You should see the health status of the API. If the page doesn't load, check the revision logs in the Azure portal under **Monitoring** > **Log stream**.

## Clean up resources

When you're finished, delete the resource group to avoid ongoing charges. Deleting the resource group removes all resources that it contains, and this action can't be undone.

## Next steps

- Learn how to [scale your app](scale.md) based on HTTP traffic or queue length.
- Set up a CI/CD pipeline with GitHub Actions to deploy every change automatically.
