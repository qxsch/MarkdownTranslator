# Deployment checklist

Complete the following tasks before a production deployment:

- Infrastructure
  - Check the quota in the target region:
    - CPU cores
    - Public IP addresses
    - Storage accounts
  - Confirm that the network rules allow:
    1. Outbound traffic on port 443
    2. Inbound traffic from the load balancer only
- Application
  - Run the test suite. All of the following must pass:
    - Unit tests
    - Integration tests, including the slow ones
  - Tag the release, for example `v3.2.0`
- People
  - Inform the support team at least one day in advance
  - Name an on-call engineer who can:
    - Roll back the release
    - Contact the customer

After the deployment, check these items:

1. The health endpoint `/healthz` returns `200`.
2. Error rates stay below the baseline for 30 minutes.
3. No alerts fire.
   - If an alert fires, follow the runbook.
   - If the runbook doesn't help, roll back.
