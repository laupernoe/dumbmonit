# Roteiro

O que está entregue, em curso e planeado é acompanhado no painel público do
projeto: [DumbMonit Roadmap](https://github.com/users/laupernoe/projects/2).
As prioridades e os prazos podem mudar; o painel é um plano vivo, não uma
promessa.

- **As alterações entregues** estão no
  [registo de alterações](https://github.com/laupernoe/dumbmonit/blob/main/CHANGELOG.md)
  e nas notas de cada
  [versão no GitHub](https://github.com/laupernoe/dumbmonit/releases).
- **As ideias e os erros** ficam no
  [gestor de issues](https://github.com/laupernoe/dumbmonit/issues). Abra uma
  issue antes de escrever código para algo de grande dimensão.
- **As correções de segurança** são anunciadas nas notas de versão e como
  avisos de segurança do GitHub, com o respetivo identificador CVE ou GHSA.

O DumbMonit está antes da 1.0: espere que a API e o modelo de dados continuem a
evoluir até ser anunciada uma linha estável. Consulte o
[guia de contribuição](https://github.com/laupernoe/dumbmonit/blob/main/CONTRIBUTING.md)
e a
[governação](https://github.com/laupernoe/dumbmonit/blob/main/GOVERNANCE.md)
para participar.

## Direção para os próximos doze meses {#direction-for-the-next-twelve-months}

Estas são intenções, não compromissos, por ordem aproximada.

- **Rumo à 1.0.** Fixar a API HTTP e o esquema da base de dados para que possa
  ser anunciada uma linha estável; até lá, as versões menores ainda os podem
  alterar, com notas de atualização no registo de alterações.
- **Mais pessoas com acesso.** Um segundo administrador do repositório, do
  registo de imagens, do alojamento da documentação e dos domínios (veja a
  [governação](https://github.com/laupernoe/dumbmonit/blob/main/GOVERNANCE.md)).
- **Cadeia de fornecimento.** Compilações de versão reprodutíveis
  ([estado atual](reproducible-builds.md)) e etiquetas de versão assinadas.
- **Imagens arm64.** As imagens do servidor são só amd64 por agora; o arm64
  regressa com executores de compilação nativos.
- **Integrações e pacotes.** Mais integrações de dispositivos e serviços e
  perfis SNMP, guiados por contribuições e pedidos.
- **Documentação.** Manter os nove idiomas da interface e da documentação em
  sintonia com a fonte em inglês.

## O que o DumbMonit não pretende fazer {#what-dumbmonit-does-not-intend-to-do}

- Tornar-se uma plataforma de logs, traços ou APM: monitoriza dispositivos e
  serviços.
- Oferecer uma edição paga ou um serviço alojado: continua 100 % código
  aberto.
- Enviar telemetria da sua instância para qualquer lado.
- Visar frotas de milhares de dispositivos: foi feito para homelabs e pequenas
  equipas, num só contentor.
