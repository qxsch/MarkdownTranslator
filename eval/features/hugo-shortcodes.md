---
title: "Architecture"
weight: 20
---

# Architecture

{{< figure src="images/architecture.png" title="How the client talks to the service" alt="Architecture diagram" >}}

The client uploads changes to the gateway. The gateway writes them to the queue.

{{% notice info %}}
The gateway is stateless, so you can run as many instances as you need.
{{% /notice %}}

{{< tabs >}}
{{< tab name="Linux" >}}
Install the package with `apt install ctsync`.
{{< /tab >}}
{{< tab name="Windows" >}}
Install the package with `winget install Contoso.Sync`.
{{< /tab >}}
{{< /tabs >}}

{{< youtube id="abc123" title="Architecture walkthrough" >}}

See also {{< ref "operations/scaling.md" >}} for scaling guidance.
