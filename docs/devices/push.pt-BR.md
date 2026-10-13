# Heartbeat (push) {#heartbeat-push}

A falha silenciosa que ninguém nota: o backup noturno que parou de rodar
depois de uma atualização, o cron de renovação de certificado que perdeu
suas permissões, a automação do Home Assistant que quebrou com uma
renomeação. Nada reclama, e você descobre no dia em que precisa do backup.

Um monitor **heartbeat** (também chamado de *dead man's switch*, ou um
monitor *push* no Uptime Kuma) inverte a lógica: o DumbMonit não contata
nada. O job chama uma URL secreta cada vez que roda, e é a **ausência** de
uma chamada que gera o alerta.

## Como funciona {#how-it-works}

Cada dispositivo heartbeat recebe uma URL da forma
`https://<seu DumbMonit>/api/push/<token>`. O token é um valor aleatório
de 128 bits, armazenado hasheado para busca e criptografado para exibição:
é mostrado na página do dispositivo, com um botão de copiar, enquanto o
dispositivo existir. É deliberadamente aberto — nenhuma sessão, nenhum
token de API, nenhum cabeçalho `X-Requested-With` — porque uma linha
`curl` em um crontab não tem nenhum desses. Possuir o token só permite
dizer "o job rodou".

A cada passagem do agendador (o intervalo do dispositivo, 60 s por
padrão) o coletor lê o horário da última chamada e escreve:

| Situação | `probe_success` | Efeito |
|---|---|---|
| Nenhuma chamada recebida ainda | *(nada escrito)* | O dispositivo mostra **Esperando**. Nenhum alerta: o job ainda não está no lugar. |
| Última chamada dentro do intervalo esperado + tolerância | `1` | **Reportando**. |
| Última chamada mais antiga que o intervalo esperado + tolerância | `0`, `reason="missed"` | **Fora do ar**; a regra "Heartbeat missed" dispara depois de dois minutos. |
| Última chamada disse `status=down` | `0`, `reason="reported_down"` | **Fora do ar** imediatamente, qualquer que seja o horário. |

Uma chamada também reavalia o dispositivo imediatamente, então um job que
retoma limpa seu alerta sem esperar pela próxima passagem do agendador.

O endpoint responde `204` com um corpo vazio quando a chamada é
registrada, `404` para um token desconhecido ou regenerado, `400` para um
`status` que não é nem `up` nem `down`, e `429` (com `Retry-After`) além
de sessenta chamadas por minuto para um token. Tanto `GET` quanto `POST`
são aceitos; `HEAD` também funciona.

### Parâmetros de consulta {#query-parameters}

| Parâmetro | Significado |
|---|---|
| `status` | `up` (padrão) ou `down`: o job reporta uma falha ele mesmo. |
| `msg` | Uma palavra livre de explicação, mantida (truncada a 500 caracteres) e mostrada na página do dispositivo. |

Esses são os parâmetros do monitor push do Uptime Kuma: um script escrito
para ele funciona sem alteração.

## Configuração {#setup}

Os passos abaixo são os que o aviso ao lado do formulário mostra.

1. No endereço, escreva um rótulo curto para o job ("nightly-backup",
   "certbot-renew"): nada é contatado, o rótulo só precisa ser único
   entre seus heartbeats.
2. Defina o intervalo esperado para a programação do job ("24h" para um
   job noturno, "1h" para um de hora em hora) e, se a programação varia,
   um período de tolerância mais amplo. Salve o dispositivo.
3. A página do dispositivo mostra a URL a chamar. Adicione-a no final do
   job, para que só seja chamada quando o job tiver sucesso:

    ```
    curl -fsS -m 10 --retry 3 https://monit.example.com/api/push/<token>
    ```

4. Um job que pode dizer quando falhou pode dizer isso em vez de ficar
   silencioso: adicione ?status=down&msg=… à URL, e o alerta dispara
   imediatamente.
5. Até a primeira chamada chegar o dispositivo mostra "Esperando" e nada
   é alertado. URL perdida ou exposta? "Regenerar" na página do
   dispositivo emite uma nova; a antiga para de responder imediatamente.

!!! warning
    Chame a URL no final do job, depois da parte que importa. Uma chamada
    colocada no topo reportaria um sucesso mesmo quando o próprio backup
    falhasse.

Credenciais: nenhuma.

### Exemplos {#examples}

Uma linha crontab, à noite às 03:00, que só chama quando o backup tem
sucesso:

```
0 3 * * * /usr/local/bin/backup.sh && curl -fsS -m 10 --retry 3 https://monit.example.com/api/push/<token> > /dev/null
```

Um script que reporta sua própria falha, com o código de saída como
mensagem:

```sh
#!/bin/sh
URL="https://monit.example.com/api/push/<token>"
if /usr/local/bin/backup.sh; then
    curl -fsS -m 10 --retry 3 "$URL" > /dev/null
else
    curl -fsS -m 10 --retry 3 "$URL?status=down&msg=backup+exit+$?" > /dev/null
fi
```

Home Assistant, um `rest_command` chamado ao final de uma automação:

```yaml
rest_command:
  dumbmonit_heartbeat:
    url: "https://monit.example.com/api/push/<token>"
    method: post
```

Windows Task Scheduler (PowerShell):

```powershell
Invoke-WebRequest -UseBasicParsing -TimeoutSec 10 "https://monit.example.com/api/push/<token>" | Out-Null
```

### Opções {#options}

| Chave | Rótulo | Padrão | Ajuda |
|---|---|---|---|
| `expected_interval` | Intervalo esperado | `24h` | Com que frequência o job deve chamar: 30m, 1h, 6h, 24h, 7d (ou um número de segundos). Uma chamada perdida é declarada uma vez que esse intervalo mais o período de tolerância tenha passado. |
| `grace` | Período de tolerância | `10%` | Tempo extra tolerado depois do intervalo esperado antes do heartbeat contar como perdido: uma porcentagem do intervalo (10%) ou uma duração fixa (15m). Nunca menos que um minuto. |

Durações aceitam sufixos `s`, `m`, `h`, `d`, `w` e podem ser combinadas
(`1h30m`); um número simples é um número de segundos. O intervalo
esperado precisa estar entre 10 segundos e um ano.

## Métricas {#metrics}

Todas rotuladas `probe="push"` além de `target`, `host` e `tag_*`, todas
com o prefixo `dumbmonit_`.

| Métrica | Significado |
|---|---|
| `probe_success` | 1 se a última chamada está no horário e não reportou uma falha, 0 caso contrário. Ausente até a primeira chamada. |
| `probe_failure_info` | Presença (1) com um rótulo `reason`: `missed` ou `reported_down`. |
| `push_last_seen_seconds` | Idade da última chamada. |
| `push_received_total` | Chamadas recebidas desde que o token foi criado (contador). |

Disponibilidade em trinta dias, como para qualquer monitor de serviço:
`avg_over_time(dumbmonit_probe_success{probe="push"}[30d])`.

Regra integrada que se aplica: **Heartbeat missed** (aviso, 2 minutos). As
regras "Service down" e "Service flapping" ignoram heartbeats, e "Device
unreachable" nunca dispara para eles: o coletor sempre roda, só seu
veredito muda.

## API {#api}

| Rota | Sessão | Propósito |
|---|---|---|
| `GET` ou `POST /api/push/<token>` | nenhuma | A própria chamada. `204`, `404`, `400` ou `429`. |
| `GET /api/targets/<id>/push` | obrigatória | O monitor: `token`, `path`, `last_seen_at`, `last_seen_age_secs`, `last_status`, `last_message`, `received_total`, `expected_interval_secs`, `grace_secs`, `verdict` (`waiting`, `on_time`, `missed`, `reported_down`), `settings_error`. Criado na primeira leitura se faltando. |
| `POST /api/targets/<id>/push/regenerate` | admin | Novo token; a URL anterior responde `404` a partir de então. O contador e a última chamada são mantidos. |
