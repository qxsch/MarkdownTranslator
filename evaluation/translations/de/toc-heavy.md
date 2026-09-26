# <a id="security-baseline"></a>Sicherheitsbaseline

## <a id="contents"></a>Inhalt

- [Übersicht](#overview)
- [Identität](#identity)
  - [Mehrstufige Authentifizierung](#multifactor-authentication)
  - [Bedingter Zugriff](#conditional-access)
- [Datenschutz](#data-protection)
  - [Verschlüsselung](#encryption)
  - [Freigabelinks](#sharing-links)
- [Überwachung](#monitoring)

## <a id="overview"></a>Übersicht

Diese Sicherheitsbaseline führt die empfohlenen Sicherheitseinstellungen für Contoso Sync auf.

## <a id="identity"></a>Identität

### <a id="multifactor-authentication"></a>Mehrstufige Authentifizierung

Fordern Sie für alle Benutzer mehrstufige Authentifizierung an.

### <a id="conditional-access"></a>Bedingter Zugriff

Lassen Sie Anmeldungen nur von verwalteten Geräten zu.

## <a id="data-protection"></a>Datenschutz

### <a id="encryption"></a>Verschlüsselung

Die Verschlüsselung ruhender Daten ist immer aktiviert. Kundenseitig verwaltete Schlüssel sind optional.

### <a id="sharing-links"></a>Freigabelinks

Legen Sie den Standardlinktyp auf **Personen in Ihrer Organisation** fest. Anonyme Links sollten nach 7 Tagen ablaufen.

## <a id="monitoring"></a>Überwachung

Senden Sie das Überwachungsprotokoll an Ihr SIEM. Überprüfen Sie [Freigabelinks](#sharing-links) monatlich.
