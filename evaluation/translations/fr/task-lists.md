# <a id="migration-tasks"></a>Tâches de migration

## <a id="before-the-migration"></a>Avant la migration

- [x] Exporter la liste des partages
- [x] Informer tous les utilisateurs par e-mail
- [ ] Supprimer les partages que personne n’a utilisés au cours de l’année écoulée
- [ ] Vérifier que chaque partage a un propriétaire

## <a id="during-the-migration"></a>Pendant la migration

- [ ] Geler les modifications sur l’ancien serveur
- [ ] Exécuter `ctsync migrate --source \\oldserver\shares`
  - [ ] Surveiller la progression dans le centre d’administration
  - [ ] Noter tout fichier en échec
- [ ] Comparer le nombre de fichiers

## <a id="after-the-migration"></a>Après la migration

1. [ ] Éteindre l’ancien serveur au bout de deux semaines
2. [ ] Archiver le journal de migration `migrate.log`
3. [x] Célébrer
