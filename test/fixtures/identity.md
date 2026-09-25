---
title: Getting started with the translator
description: "Learn how to deploy the service: step by step"
author: Jane Doe
tags: [azure, markdown]
---

# Getting started with `mdtranslator`

This guide explains how to **deploy** the service and how to *configure* it for [Azure](https://azure.microsoft.com "Azure home page").

See the file test.jpeg, the folder `docs/images/` and ./scripts/deploy.ps1 before you continue.

![Architecture diagram of the solution](images/architecture.png "High level architecture")

Use the [reference link][docs] or visit <https://learn.microsoft.com> for details, e.g. www.example.com.

> **Note:** Values such as `$HOME`, %APPDATA% and ${VAR_NAME} are never translated.

## Prerequisites

- An Azure subscription with the `Contributor` role
- [x] Node.js 22 or later
- [ ] Docker Desktop 4.30 or later, see README.md

1. Clone the repository.
2. Run `npm install` in the project folder.
3. Start the server with --port 8080 and the -v flag.

| Setting | Default | Description |
|:--------|:-------:|------------:|
| `PORT` | 8080 | Port the HTTP server listens on |
| `MDT_REVIEW` | true | Enables the second review pass \| optional |

Press <kbd>Ctrl</kbd>+<kbd>C</kbd> to stop the server, or read the <strong>important</strong> notes below.

<div align="center">
  <img src="images/logo.svg" alt="Company logo" width="120">
  <p>This paragraph is inside an <em>HTML block</em> and should be translated.</p>
</div>

<!-- This HTML comment stays untouched -->

```typescript
// Create the client for the translation service
const client = new TranslatorClient(endpoint); // inline comment
/* Multi-line block
   comment explaining the retry policy */
const url = "https://example.com // not a comment";
```

```python
def main():
    # Read the configuration file first
    cfg = load("config.yaml")  # load settings
    return cfg
```

```bash
#!/usr/bin/env bash
# Install the dependencies
npm ci # quiet install
echo "# not a comment"
```

```powershell
<#
  Deploys the infrastructure to the resource group.
#>
New-AzResourceGroupDeployment -ResourceGroupName rg -TemplateFile main.bicep # deploy
```

```bicep
// Storage account for the translation cache
param location string = resourceGroup().location
```

```json
{ "name": "no comments in json" }
```

Here is a footnote reference[^1] and a hard line break at the end of this line\
followed by the next line.

Entities like &copy; and &nbsp; and escaped \*stars\* stay as they are.

[docs]: https://learn.microsoft.com/azure "Azure documentation"
[^1]: The footnote text is translated as well.
