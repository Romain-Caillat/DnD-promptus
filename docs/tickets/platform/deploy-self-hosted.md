# platform/deploy-self-hosted · Héberger Promptus chez Romain

**État** : en cours · reste le choix de l'hôte et la mise en ligne

**Réalise** : [Bible, « Les rôles »](../../bible.md)

**Objectif** : Les joueurs rejoignent depuis chez eux, sans VPN.

**Périmètre** :

- fait et vérifié dans PCT 105 : image Docker en trois étages, le serveur sert aussi le front
- fait : Compose de prod (app + Postgres 17), migrations au démarrage, santé, install, sauvegarde chiffrable, restauration
- reste pour Romain : choisir l'hôte (proposition dans `docs/install.md`) et y installer
- reste : route sur le Traefik de PCT 100, DNS chez Ionos, boucle de déploiement sur l'hôte
- `prod-deploy.sh` n'a pas tourné, faute de conteneur cible

**Fini quand** : Un téléphone en 4G ouvre un lien d'invitation et rejoint (attend aussi `session/invite-and-join`) ; une sauvegarde se restaure sur une base vide.

**Départ** : `archive/tickets/backlog-avant-bible.md`, `docs/install.md`, `docs/backup.md`
