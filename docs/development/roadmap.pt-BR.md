# Roadmap

O que foi entregue, o que está em andamento e o que está planejado é acompanhado
no quadro público do projeto: [DumbMonit Roadmap](https://github.com/users/laupernoe/projects/2).
Prioridades e prazos podem mudar; o quadro é um plano vivo, não uma promessa.

- **As mudanças entregues** estão no
  [changelog](https://github.com/laupernoe/dumbmonit/blob/main/CHANGELOG.md)
  e nas notas de cada
  [lançamento do GitHub](https://github.com/laupernoe/dumbmonit/releases).
- **Ideias e bugs** ficam no
  [rastreador de issues](https://github.com/laupernoe/dumbmonit/issues). Abra
  uma issue antes de escrever código para algo grande.
- **Correções de segurança** são anunciadas nas notas de lançamento e como
  avisos de segurança do GitHub, com seu identificador CVE ou GHSA.

O DumbMonit está antes da 1.0: espere que a API e o modelo de dados continuem
evoluindo até que uma linha estável seja anunciada. Veja o
[guia de contribuição](https://github.com/laupernoe/dumbmonit/blob/main/CONTRIBUTING.md)
e a
[governança](https://github.com/laupernoe/dumbmonit/blob/main/GOVERNANCE.md)
para participar.

## Direção para os próximos doze meses {#direction-for-the-next-twelve-months}

São intenções, não compromissos, em ordem aproximada.

- **Rumo à 1.0.** Estabilizar a API HTTP e o esquema do banco de dados para que
  uma linha estável possa ser anunciada; até lá, versões menores ainda podem
  alterá-los, com notas de atualização no changelog.
- **Mais pessoas com acesso.** Um segundo administrador do repositório, do
  registro, da hospedagem da documentação e dos domínios (veja a
  [governança](https://github.com/laupernoe/dumbmonit/blob/main/GOVERNANCE.md)).
- **Cadeia de suprimentos.** Builds de release reproduzíveis
  ([estado atual](reproducible-builds.md)) e tags de versão assinadas.
- **Imagens arm64.** As imagens do servidor são apenas amd64 por enquanto; o
  arm64 volta com runners de build nativos.
- **Integrações e pacotes.** Mais integrações de dispositivos e serviços e
  perfis SNMP, guiados por contribuições e pedidos.
- **Documentação.** Manter os nove idiomas da interface e da documentação em
  sincronia com a fonte em inglês.

## O que o DumbMonit não pretende fazer {#what-dumbmonit-does-not-intend-to-do}

- Tornar-se uma plataforma de logs, traces ou APM: ele monitora dispositivos e
  serviços.
- Oferecer uma edição paga ou um serviço hospedado: ele continua 100 % de
  código aberto.
- Enviar telemetria sobre a sua instância para qualquer lugar.
- Mirar frotas de milhares de dispositivos: ele é feito para homelabs e
  pequenas equipes, em um só contêiner.
