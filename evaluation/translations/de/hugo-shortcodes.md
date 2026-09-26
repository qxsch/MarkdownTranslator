---
title: "Architektur"
weight: 20
---

# <a id="architecture"></a>Architektur

{{< figure src="images/architecture.png" title="Wie der Client mit dem Dienst kommuniziert" alt="Architekturdiagramm" >}}

Der Client lädt Änderungen auf das Gateway hoch. Das Gateway schreibt sie in die Warteschlange.

{{% notice info %}}
Das Gateway ist zustandslos, sodass Sie so viele Instanzen ausführen können, wie Sie benötigen.
{{% /notice %}}

{{< tabs >}}
{{< tab name="Linux" >}}
Installieren Sie das Paket mit `apt install ctsync`.
{{< /tab >}}
{{< tab name="Windows" >}}
Installieren Sie das Paket mit `winget install Contoso.Sync`.
{{< /tab >}}
{{< /tabs >}}

{{< youtube id="abc123" title="Einführung in die Architektur" >}}

Hinweise zur Skalierung finden Sie auch unter {{< ref "operations/scaling.md" >}}.
