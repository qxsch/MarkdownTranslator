# Security baseline

## Contents

- [Overview](#overview)
- [Identity](#identity)
  - [Multifactor authentication](#multifactor-authentication)
  - [Conditional access](#conditional-access)
- [Data protection](#data-protection)
  - [Encryption](#encryption)
  - [Sharing links](#sharing-links)
- [Monitoring](#monitoring)

## Overview

This baseline lists the recommended security settings for Contoso Sync.

## Identity

### Multifactor authentication

Require multifactor authentication for all users.

### Conditional access

Allow sign-in only from managed devices.

## Data protection

### Encryption

Encryption at rest is always on. Customer-managed keys are optional.

### Sharing links

Set the default link type to **People in your organization**. Anonymous links should expire after 7 days.

## Monitoring

Send the audit log to your SIEM. Review [sharing links](#sharing-links) every month.
