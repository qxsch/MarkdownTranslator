---
title: "Architecture"
weight: 20
---

# Architecture

{{< figure src="images/architecture.png" title="Comment le client communique avec le service" alt="Diagramme d’architecture" >}}

Le client téléverse les modifications vers la passerelle. La passerelle les écrit dans la file d’attente.

{{% notice info %}}
La passerelle est sans état ; vous pouvez donc exécuter autant d’instances que nécessaire.
{{% /notice %}}

{{< tabs >}}
{{< tab name="Linux" >}}
Installez le paquet avec `apt install ctsync`.
{{< /tab >}}
{{< tab name="Windows" >}}
Installez le paquet avec `winget install Contoso.Sync`.
{{< /tab >}}
{{< /tabs >}}

{{< youtube id="abc123" title="Présentation de l’architecture" >}}

Voir aussi {{< ref "operations/scaling.md" >}} pour obtenir des conseils de mise à l’échelle.
