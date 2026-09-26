---
title: "Architecture"
weight: 20
---

# Architecture

{{< figure src="images/architecture.png" title="Comment le client communique avec le service" alt="Schéma d’architecture" >}}

Le client envoie les modifications à la passerelle. La passerelle les écrit dans la file d’attente.

{{% notice info %}}
La passerelle étant sans état, vous pouvez exécuter autant d’instances que nécessaire.
{{% /notice %}}

{{< tabs >}}
{{< tab name="Linux" >}}
Installez le package avec `apt install ctsync`.
{{< /tab >}}
{{< tab name="Windows" >}}
Installez le package avec `winget install Contoso.Sync`.
{{< /tab >}}
{{< /tabs >}}

{{< youtube id="abc123" title="Présentation de l’architecture" >}}

Consultez également {{< ref "operations/scaling.md" >}} pour obtenir des recommandations de mise à l’échelle.
