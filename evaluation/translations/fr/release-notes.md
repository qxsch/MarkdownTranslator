# <a id="release-notes"></a>Notes de publication

## Version 3.2.0 (2026-08-14)

### <a id="new-features"></a>Nouvelles fonctionnalités

- **Synchronisation sélective des sous-dossiers.** Vous pouvez désormais exclure des sous-dossiers individuels au lieu de partages entiers.
- La nouvelle option `--dry-run` pour `ctsync reset` affiche ce qui serait supprimé.
- Le mode sombre suit le paramètre système.

### <a id="fixed-issues"></a>Problèmes résolus

- Correction d’un plantage lorsqu’un nom de fichier contenait un emoji.
- L’icône de la zone de notification ne scintille plus pendant les téléversements volumineux.
- Renommer un dossier pendant sa synchronisation créait un doublon. Ce problème est résolu.

### <a id="known-issues"></a>Problèmes connus

- Sur macOS 14.1, les notifications peuvent apparaître deux fois. Mettez à jour vers 14.2 pour corriger ce problème.

## Version 3.1.4 (2026-06-02)

- Mise à jour de sécurité pour CVE-2026-1234. Effectuez la mise à jour dès que possible.
- Amélioration de la vitesse de téléversement jusqu’à 30 % sur les connexions lentes.

## Version 3.1.0 (2026-04-20)

### <a id="breaking-changes"></a>Modifications avec rupture

- Le fichier de configuration est passé de `config.ini` à `config.toml`. L’application le migre automatiquement au premier démarrage.
- La prise en charge de Windows 10 version 1809 a pris fin.
