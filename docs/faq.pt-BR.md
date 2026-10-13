# Perguntas frequentes {#faq}

## Perdi a senha {#i-lost-the-password}

Peça a outro administrador primeiro: em Configurações → **Usuários**,
**Editar** na sua conta define uma nova senha e a mostra uma vez, e
**Redefinir 2FA** na mesma linha limpa um segundo fator que o trancou fora.
Essa é a resposta do dia a dia — uma instância com duas contas de
administrador nunca precisa do resto desta página.

Quando ninguém mais consegue entrar, não há e-mail de recuperação: inicie o
servidor uma vez com `DUMBMONIT_RESET_PASSWORD=1`, definida no `docker-compose.yml`:

```yaml
    environment:
      DUMBMONIT_RESET_PASSWORD: "1"
```

Depois, `docker compose up -d`.

Toda conta e toda sessão são removidas na inicialização e a interface mostra
`/setup` de novo. Crie o primeiro administrador, depois inicie de novo sem essa linha (remova-a e execute `docker compose up -d` de novo). Dispositivos, regras, canais e as configurações de
login único permanecem intactos.

## Sem dados após adicionar um dispositivo {#no-data-after-adding-a-device}

Dê um minuto: a primeira sondagem roda no intervalo do dispositivo (60 s por
padrão) e escritas chegam ao VictoriaMetrics a cada 5 segundos
(`DUMBMONIT_FLUSH_INTERVAL_SECS`). Depois:

1. Abra a página do dispositivo e clique em **Sondar agora**. Ela reporta
   quantas amostras e séries uma sondagem produziu, ou o erro exato.
2. *Mal configurado* com um erro abaixo do endereço é um problema de
   configuração: comunidade ou senha errada, certificado recusado (ative
   "Aceitar um certificado não verificável"), capacidade faltando. É
   mostrado, não alertado.
3. *Inacessível* sem erro significa que o dispositivo não respondeu de
   forma alguma. Para SNMP, uma comunidade errada parece exatamente um
   dispositivo desligado, porque o dispositivo nunca responde a ela.
   Verifique a comunidade, que o SNMP está ativado, e que o dispositivo
   permite o endereço do servidor.
4. Verifique que o contêiner pode alcançar o dispositivo: redes Docker e
   firewalls são os culpados habituais.

## Ping diz "erro de configuração" {#ping-says-configuration-error}

O monitor de ping precisa de um socket de echo ICMP. O contêiner roda sem
nenhuma capacidade, então o arquivo Compose permite esses sockets com o
sysctl `net.ipv4.ping_group_range` sob o serviço `dumbmonit`: certifique-se
de que as linhas `sysctls:` ainda estão lá, depois `docker compose up -d`.
Veja
[Ping ICMP sem uma capacidade](install/docker.md#icmp-ping-without-a-capability).

## O agente não aparece {#the-agent-does-not-appear}

- O instalador faz um envio de teste antes de iniciar o serviço e para com
  um erro explícito se a URL ou o token estiverem errados. Corrija
  `/etc/dumbmonit/agent.yaml` e reinicie `dumbmonit-agent`.
- A URL no comando de instalação é a que seu navegador usou para alcançar a
  interface. A partir da máquina monitorada, essa URL precisa alcançar o
  servidor: `curl http://server:8080/api/health` a partir de lá informa
  isso.
- Detrás de um proxy reverso, `/api/ingest` precisa ser encaminhado e
  corpos de até 16 MB permitidos (`client_max_body_size 16m` no nginx).
- `journalctl -u dumbmonit-agent -f` mostra todo envio recusado. O próprio
  token nunca aparece no log.
- Um token revogado para o agente no próximo envio; execute o instalador de
  novo com um novo token.

Veja [Agente Linux, macOS, FreeBSD e Windows](devices/agent.md).

## Mudando a porta {#changing-the-port}

O contêiner escuta na 8080; a porta do host é o lado esquerdo do mapeamento em
`docker-compose.yml`. Edite-o e execute `docker compose up -d`:

```yaml
    ports:
      - "8099:8080"
```

Para mudar a porta *dentro* do contêiner, defina `DUMBMONIT_BIND`, por
exemplo `0.0.0.0:9000`, e ajuste o mapeamento de porta.

## Por que o alerta é "Aviso" quando a regra diz "warning"? {#why-is-the-alert-advisory-when-the-rule-says-warning}

A API armazena três severidades: `info`, `warning`, `critical`. A interface
as mostra com as palavras do boletim meteorológico: Info, Aviso, Alerta.
Veja [Severidades](alerting/index.md#severities).

## Um alerta está "Suprimido pelo pai" {#an-alert-is-suppressed-by-parent}

O dispositivo tem um dispositivo pai inacessível, então seus próprios
alertas são retidos em vez de enviados. Corrija o pai; os alertas do filho
se resolvem ou disparam por conta própria depois, sem uma segunda
notificação. Veja
[Supressão por dependência](alerting/index.md#dependency-suppression).

## "CPU inusual" nunca dispara {#unusual-cpu-never-fires}

É uma linha de base sazonal que permanece silenciosa por seus primeiros 14
dias em cada série. Durante esse tempo ela só mostra o que teria disparado.
Veja [Aprendizado de linha de base](alerting/index.md#baseline-learning).

## Posso usar meu próprio VictoriaMetrics? {#can-i-use-my-own-victoriametrics}

Sim: defina `DUMBMONIT_VM_URL` para seu endereço (`http://host:8428`) e o
VictoriaMetrics embutido não é iniciado. Toda métrica tem o prefixo
`dumbmonit_`, então pode compartilhar a instância com outras ferramentas.
`GET /api/health` reporta `victoria.embedded: false` uma vez feita a
mudança.

## O DumbMonit precisa ser acessível pela internet? {#does-dumbmonit-need-to-be-reachable-from-the-internet}

Não. Ele alcança dispositivos e serviços de notificação; nada chama de
volta, exceto agentes, que precisam alcançar a URL do servidor. Mantenha-o
na sua LAN ou detrás de uma VPN; se você o expõe, use um proxy reverso com
TLS e defina `DUMBMONIT_COOKIE_SECURE=1`.

## Existe um token de API? {#is-there-an-api-token}

Sim. Configurações → **API e assistentes** cria tokens `dmt_…` com um
escopo `read` ou `write`; envie um como `Authorization: Bearer dmt_…` em
qualquer rota e nenhum cookie ou cabeçalho CSRF é necessário. O mesmo token
conecta um [assistente](using/assistant.md) via MCP. Tokens nunca gerenciam
contas, configurações de login ou outros tokens. Veja a
[referência da API HTTP](reference/api.md#authentication).
