# Heartbeat (push) {#heartbeat-push}

A falha silenciosa que ninguém nota: a cópia de segurança noturna que
parou de correr depois de uma atualização, o cron de renovação de
certificado que perdeu as suas permissões, a automação Home Assistant que
se avariou numa renomeação. Nada se queixa, e descobre-o no dia em que
precisa da cópia de segurança.

Um monitor **heartbeat** (também chamado *dead man's switch*, ou um
monitor *push* no Uptime Kuma) inverte a lógica: o DumbMonit não contacta
nada. A tarefa chama um URL secreto cada vez que corre, e é a **ausência**
de uma chamada que gera o alerta.

## Como funciona {#how-it-works}

Cada monitor heartbeat recebe um URL da forma
`https://<o seu DumbMonit>/api/push/<token>`. O token é um valor aleatório
de 128 bits, guardado com hash para procura e encriptado para exibição: é
mostrado na página do monitor, com um botão de cópia, enquanto o
monitor existir. É deliberadamente aberto — sem sessão, sem token de
API, sem cabeçalho `X-Requested-With` — porque uma linha `curl` numa
crontab não tem nada disso. Ter o token só permite dizer «a tarefa
correu».

A cada passagem do escalonador (o intervalo do monitor, 60 s por
predefinição) o coletor lê a hora da última chamada e escreve:

| Situação | `probe_success` | Efeito |
|---|---|---|
| Nenhuma chamada recebida ainda | *(nada escrito)* | O monitor mostra **Waiting**. Nenhum alerta: a tarefa ainda não está em funcionamento. |
| Última chamada dentro do intervalo esperado + margem | `1` | **Reporting**. |
| Última chamada mais antiga do que o intervalo esperado + margem | `0`, `reason="missed"` | **Down**; a regra «Heartbeat missed» dispara depois de dois minutos. |
| A última chamada disse `status=down` | `0`, `reason="reported_down"` | **Down** de imediato, seja o que for que o relógio diga. |

Uma chamada também reavalia o monitor imediatamente, por isso uma
tarefa que retoma limpa o seu alerta sem esperar pela próxima passagem do
escalonador.

O endpoint responde `204` com um corpo vazio quando a chamada é
registada, `404` para um token desconhecido ou regenerado, `400` para um
`status` que não é `up` nem `down`, e `429` (com `Retry-After`) acima de
sessenta chamadas por minuto para um token. Tanto `GET` como `POST` são
aceites; `HEAD` também funciona.

### Parâmetros de consulta {#query-parameters}

| Parâmetro | Significado |
|---|---|
| `status` | `up` (predefinição) ou `down`: a própria tarefa reporta uma falha. |
| `msg` | Uma palavra livre de explicação, guardada (truncada a 500 caracteres) e mostrada na página do monitor. |

Estes são os parâmetros do monitor push do Uptime Kuma: um script escrito
para ele funciona sem alterações.

## Configuração {#setup}

Os passos abaixo são os que o aviso junto ao formulário mostra.

1. No endereço, escreva um rótulo curto para a tarefa («nightly-backup»,
   «certbot-renew»): nada é contactado, o rótulo só tem de ser único entre
   os seus heartbeats.
2. Defina o intervalo esperado para o calendário da tarefa («24h» para
   uma tarefa noturna, «1h» para uma horária) e, se o calendário derivar,
   um período de margem mais largo. Guarde o monitor.
3. A página do monitor mostra o URL a chamar. Adicione-o no fim da
   tarefa, para que só seja chamado quando a tarefa tiver sucesso:

    ```
    curl -fsS -m 10 --retry 3 https://monit.example.com/api/push/<token>
    ```

4. Uma tarefa que consegue saber quando falhou pode dizê-lo em vez de
   ficar silenciosa: acrescente ?status=down&msg=… ao URL, e o alerta
   dispara de imediato.
5. Até a primeira chamada chegar, o monitor mostra «Waiting» e nada é
   alertado. URL perdido ou exposto? «Regenerate» na página do monitor
   emite um novo; o antigo deixa de responder de imediato.

!!! warning
    Chame o URL no fim da tarefa, depois da parte que importa. Uma
    chamada colocada no topo reportaria um sucesso mesmo quando a própria
    cópia de segurança falhasse.

Credenciais: nenhuma.

### Exemplos {#examples}

Uma linha de crontab, à noite às 03:00, que só chama quando a cópia de
segurança tem sucesso:

```
0 3 * * * /usr/local/bin/backup.sh && curl -fsS -m 10 --retry 3 https://monit.example.com/api/push/<token> > /dev/null
```

Um script que reporta a sua própria falha, com o código de saída como
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

Home Assistant, um `rest_command` chamado no fim de uma automação:

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

| Chave | Rótulo | Predefinição | Ajuda |
|---|---|---|---|
| `expected_interval` | Expected interval | `24h` | Com que frequência a tarefa deve chamar: 30m, 1h, 6h, 24h, 7d (ou um número de segundos). Uma chamada em falta é declarada depois de este intervalo mais o período de margem ter passado. |
| `grace` | Grace period | `10%` | Tempo extra tolerado depois do intervalo esperado antes de o heartbeat contar como em falta: uma percentagem do intervalo (10%) ou uma duração fixa (15m). Nunca menos de um minuto. |

As durações aceitam sufixos `s`, `m`, `h`, `d`, `w` e podem ser combinadas
(`1h30m`); um número isolado é um número de segundos. O intervalo esperado
tem de estar entre 10 segundos e um ano.

## Métricas {#metrics}

Todas etiquetadas `probe="push"` além de `target`, `host` e `tag_*`, todas
com o prefixo `dumbmonit_`.

| Métrica | Significado |
|---|---|
| `probe_success` | 1 se a última chamada está a tempo e não reportou uma falha, 0 caso contrário. Ausente até à primeira chamada. |
| `probe_failure_info` | Presença (1) com uma etiqueta `reason`: `missed` ou `reported_down`. |
| `push_last_seen_seconds` | Idade da última chamada. |
| `push_received_total` | Chamadas recebidas desde que o token foi criado (contador). |

Disponibilidade ao longo de trinta dias, como para qualquer monitor de
serviço: `avg_over_time(dumbmonit_probe_success{probe="push"}[30d])`.

Regra incorporada que se aplica: **Heartbeat missed** (advisory, 2
minutos). As regras «Service down» e «Service flapping» ignoram
heartbeats, e «Monitor unreachable» nunca dispara para eles: o coletor
corre sempre, só o seu veredicto muda.

## API {#api}

| Rota | Sessão | Propósito |
|---|---|---|
| `GET` ou `POST /api/push/<token>` | nenhuma | A própria chamada. `204`, `404`, `400` ou `429`. |
| `GET /api/targets/<id>/push` | obrigatória | O monitor: `token`, `path`, `last_seen_at`, `last_seen_age_secs`, `last_status`, `last_message`, `received_total`, `expected_interval_secs`, `grace_secs`, `verdict` (`waiting`, `on_time`, `missed`, `reported_down`), `settings_error`. Criado na primeira leitura se estiver em falta. |
| `POST /api/targets/<id>/push/regenerate` | admin | Novo token; o URL anterior responde `404` a partir de então. O contador e a última chamada são mantidos. |
