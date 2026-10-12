# Perguntas frequentes {#faq}

## Perdi a senha {#i-lost-the-password}

Peça primeiro a outro administrador: em Settings → **Users**, **Edit** na
sua conta define uma nova senha e mostra-a uma vez, e **Reset 2FA** na
mesma linha limpa um segundo fator que o bloqueia. Essa é a resposta do
dia a dia — uma instância com duas contas de administrador nunca precisa
do resto desta página.

Quando ninguém mais consegue iniciar sessão, não há email de recuperação:
inicie o servidor uma vez com `DUMBMONIT_RESET_PASSWORD=1`:

```bash
DUMBMONIT_RESET_PASSWORD=1 docker compose up -d dumbmonit
```

Toda conta e toda sessão são removidas no arranque e a interface mostra
`/setup` de novo. Crie o primeiro administrador, depois inicie de novo sem
a variável (`docker compose up -d dumbmonit`, com a variável por definir
ou vazia no seu ambiente). Os dispositivos, regras, canais e as
definições de sign-on único não são tocados.

## Sem dados depois de adicionar um dispositivo {#no-data-after-adding-a-device}

Dê-lhe um minuto: a primeira verificação corre no intervalo do dispositivo
(60 s por predefinição) e as escritas chegam à VictoriaMetrics a cada 5
segundos (`DUMBMONIT_FLUSH_INTERVAL_SECS`). Depois:

1. Abra a página do dispositivo e clique em **Probe now**. Reporta quantas
   amostras e séries uma verificação produziu, ou o erro exato.
2. *Misconfigured* com um erro sob o endereço é um problema de
   configuração: comunidade ou senha errada, certificado recusado (ative
   «Accept an unverifiable certificate»), capacidade em falta. É mostrado,
   não alertado.
3. *Unreachable* sem erro significa que o dispositivo não respondeu de
   todo. Para SNMP, uma comunidade errada parece exatamente um dispositivo
   desligado, porque o dispositivo nunca lhe responde. Verifique a
   comunidade, que o SNMP está ativado, e que o dispositivo permite o
   endereço do servidor.
4. Verifique que o contentor consegue alcançar o dispositivo: redes
   Docker e firewalls são os culpados habituais.

## O ping diz «erro de configuração» {#ping-says-configuration-error}

O monitor de ping precisa de um socket de echo ICMP. O contentor corre
sem nenhuma capacidade, por isso o ficheiro Compose permite esses sockets
com o sysctl `net.ipv4.ping_group_range` sob o serviço `dumbmonit`:
certifique-se de que as linhas `sysctls:` continuam lá, depois
`docker compose up -d`. Ver
[Ping ICMP sem capacidade](install/docker.md#icmp-ping-without-a-capability).

## O agente não aparece {#the-agent-does-not-appear}

- O instalador faz um push de teste antes de iniciar o serviço e para com
  um erro explícito se o URL ou o token estiverem errados. Corrija
  `/etc/dumbmonit/agent.yaml` e reinicie `dumbmonit-agent`.
- O URL no comando de instalação é o que o seu navegador usou para
  alcançar a interface. A partir da máquina monitorizada, esse URL tem de
  alcançar o servidor: `curl http://server:8080/api/health` a partir daí
  diz-lhe isso.
- Atrás de um proxy inverso, `/api/ingest` tem de ser encaminhado e
  corpos até 16 MB permitidos (`client_max_body_size 16m` no nginx).
- `journalctl -u dumbmonit-agent -f` mostra cada push recusado. O próprio
  token nunca aparece no registo.
- Um token revogado para o agente no seu próximo push; execute o
  instalador de novo com um token novo.

Ver [Agente Linux, macOS, FreeBSD e Windows](devices/agent.md).

## Alterar a porta {#changing-the-port}

O contentor escuta na 8080; a porta do anfitrião é a que o
`docker-compose.yml` publica. Defina `DUMBMONIT_PORT` (num ficheiro `.env`
junto ao ficheiro Compose, ou na linha de comandos):

```bash
DUMBMONIT_PORT=8099 docker compose up -d
```

Para alterar a porta *dentro* do contentor, defina `DUMBMONIT_BIND`, por
exemplo `0.0.0.0:9000`, e ajuste o mapeamento de porta.

## Porque é o alerta «Advisory» quando a regra diz «warning»? {#why-is-the-alert-advisory-when-the-rule-says-warning}

A API guarda três severidades: `info`, `warning`, `critical`. A interface
mostra-as com as palavras do boletim meteorológico: Info, Advisory,
Warning. Ver [Severidades](alerting/index.md#severities).

## Um alerta está «Suppressed by parent» {#an-alert-is-suppressed-by-parent}

O dispositivo tem um dispositivo pai inacessível, por isso os seus
próprios alertas são retidos em vez de enviados. Resolva o pai; os
alertas do filho resolvem-se ou disparam por si só depois disso sem uma
segunda notificação. Ver
[Supressão por dependência](alerting/index.md#dependency-suppression).

## «Unusual CPU» nunca dispara {#unusual-cpu-never-fires}

É uma linha de base sazonal que fica silenciosa durante os seus
primeiros 14 dias em cada série. Durante esse tempo só mostra o que
teria disparado. Ver
[Aprendizagem da linha de base](alerting/index.md#baseline-learning).

## Posso usar o meu próprio VictoriaMetrics? {#can-i-use-my-own-victoriametrics}

Sim: defina `DUMBMONIT_VM_URL` para o seu endereço (`http://host:8428`) e
o VictoriaMetrics incorporado não é iniciado. Toda métrica tem o prefixo
`dumbmonit_`, por isso pode partilhar a instância com outras ferramentas.
`GET /api/health` reporta `victoria.embedded: false` uma vez feita a
mudança.

## O DumbMonit precisa de estar acessível a partir da internet? {#does-dumbmonit-need-to-be-reachable-from-the-internet}

Não. Alcança dispositivos e serviços de notificação; nada chama de
volta, exceto os agentes, que precisam de alcançar o URL do servidor.
Mantenha-o na sua LAN ou atrás de uma VPN; se o expuser, use um proxy
inverso com TLS e defina `DUMBMONIT_COOKIE_SECURE=1`.

## Existe um token de API? {#is-there-an-api-token}

Sim. Settings → **API & assistants** cria tokens `dmt_…` com um âmbito
`read` ou `write`; envie um como `Authorization: Bearer dmt_…` em
qualquer rota e não é necessário nenhum cookie nem cabeçalho CSRF. O
mesmo token liga um [assistente](using/assistant.md) via MCP. Os tokens
nunca gerem contas, definições de início de sessão ou outros tokens. Ver
a [referência da API HTTP](reference/api.md#authentication).
