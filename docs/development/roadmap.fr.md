# Feuille de route

Ce qui est livré, en cours et prévu est suivi sur le tableau de projet public :
[DumbMonit Roadmap](https://github.com/users/laupernoe/projects/2). Les
priorités et les délais peuvent changer ; le tableau est un plan vivant, pas
une promesse.

- **Les changements livrés** sont dans le
  [journal des modifications](https://github.com/laupernoe/dumbmonit/blob/main/CHANGELOG.md)
  et dans les notes de chaque
  [version GitHub](https://github.com/laupernoe/dumbmonit/releases).
- **Les idées et les bogues** vivent dans le
  [suivi des tickets](https://github.com/laupernoe/dumbmonit/issues). Ouvrez un
  ticket avant d'écrire du code pour quelque chose d'important.
- **Les correctifs de sécurité** sont annoncés dans les notes de version et
  comme avis de sécurité GitHub, avec leur identifiant CVE ou GHSA.

DumbMonit est avant la 1.0 : attendez-vous à ce que l'API et le modèle de données
évoluent encore jusqu'à l'annonce d'une version stable. Voir le
[guide de contribution](https://github.com/laupernoe/dumbmonit/blob/main/CONTRIBUTING.md)
et la
[gouvernance](https://github.com/laupernoe/dumbmonit/blob/main/GOVERNANCE.md)
pour participer.

## Orientation pour les douze prochains mois

Ce sont des intentions, pas des engagements, à peu près par ordre de priorité.

- **Vers la 1.0.** Stabiliser l'API HTTP et le schéma de la base afin de
  pouvoir annoncer une version stable ; d'ici là, les versions mineures peuvent
  encore les modifier, avec des notes de mise à jour dans le journal des
  modifications.
- **Plus de personnes avec accès.** Un second administrateur du dépôt, du
  registre, de l'hébergement de la documentation et des domaines (voir la
  [gouvernance](https://github.com/laupernoe/dumbmonit/blob/main/GOVERNANCE.md)).
- **Chaîne d'approvisionnement.** Builds de version reproductibles
  ([état actuel](reproducible-builds.md)) et tags de version signés.
- **Images arm64.** Les images du serveur sont pour l'instant amd64
  uniquement ; arm64 reviendra avec des runners de build natifs.
- **Intégrations et packs.** Davantage d'intégrations d'appareils et de
  services et de profils SNMP, selon les contributions et les demandes.
- **Documentation.** Garder les neuf langues de l'interface et de la
  documentation alignées sur la source anglaise.

## Ce que DumbMonit ne compte pas faire

- Devenir une plateforme de journaux, de traces ou d'APM : il surveille des
  appareils et des services.
- Proposer une édition payante ou un service hébergé : il reste 100 % libre.
- Envoyer de la télémétrie sur votre instance où que ce soit.
- Viser des parcs de milliers d'appareils : il est conçu pour les homelabs et
  les petites équipes, sur un seul conteneur.
