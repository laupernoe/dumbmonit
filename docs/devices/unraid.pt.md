# Unraid {#unraid}

Um servidor Unraid através da sua API GraphQL: um array que não está
iniciado, discos desativados, em falta ou a reportar erros de leitura,
verificações de paridade que encontraram erros ou não correm há semanas,
pools de cache a enchê-se, contentores que deveriam estar em execução e não
estão, e o estado das VMs.

O DumbMonit envia uma chave de API com a função Viewer no cabeçalho
`x-api-key` para `/graphql`. A função Viewer lê tudo e não pode alterar
nada. A API está incorporada no Unraid a partir da 7.2; na 6.12 a 7.1 vem
com o plugin Unraid Connect.

A cada minuto, o DumbMonit envia uma consulta obrigatória (versão, array,
paridade, discos de dados e cache) e quatro opcionais, cada uma por si só
para que um campo que uma versão mais antiga da API não tenha, ou VMs
desligadas, só deixe essa parte de fora: a verificação de paridade, os
contentores Docker, as VMs e as notificações não lidas.

!!! warning "Validado apenas contra a documentação do fornecedor — ainda não testado num sistema real"

    Esta integração foi construída a partir da documentação da API do
    Unraid e do esquema GraphQL publicado da API do Unraid (nomes de
    campos, enumerações, tamanhos em kilobytes), não contra um servidor em
    funcionamento. Toda consulta é uma leitura. Diga-nos o que falha.

## O que monitoriza {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_unraid_`.

| Métrica | Etiquetas | Significado |
|---|---|---|
| `version_info` | `version` | Valor 1: a versão do Unraid. |
| `array_started` | | 1 quando o array está iniciado. |
| `array_state_info` | `state` | Valor 1: `started`, `stopped`, `new_array`, `recon_disk`, `disable_disk`… |
| `array_size_bytes`, `array_used_bytes`, `array_free_bytes`, `array_used_percent` | | Capacidade do array. |
| `disk_ok` | `disk`, `role` (`parity`, `data`, `cache`) | 1 quando o estado do disco é `DISK_OK`. Ranhuras vazias são deixadas de fora. |
| `disk_status_info` | `disk`, `role`, `status` | Valor 1: o estado do Unraid (`disk_ok`, `disk_dsbl`, `disk_np_missing`, `disk_invalid`…). |
| `disk_errors` | `disk`, `role` | Erros de leitura desde que o array arrancou (a coluna Errors). |
| `disk_temperature_celsius` | `disk`, `role` | Só enquanto o disco gira: o Unraid não reporta nenhuma para um disco em suspensão. |
| `disk_spinning` | `disk`, `role` | 1 quando o disco gira. |
| `disk_size_bytes`, `disk_fs_size_bytes`, `disk_used_percent` | `disk`, `role` | Tamanho, tamanho do sistema de ficheiros e ocupação, para discos com um sistema de ficheiros. |
| `disks`, `disks_problem`, `disks_with_errors` | | Contagens. |
| `parity_check_status_info` | `status` | Valor 1: `completed`, `running`, `paused`, `cancelled`, `failed`, `never_run`. |
| `parity_check_running`, `parity_check_progress_percent` | | Uma verificação em curso. |
| `parity_check_ok` | | Depois de uma verificação terminada: 1 sem erros de sincronização, 0 com erros ou quando falhou. Uma verificação cancelada não prova nem define nada. |
| `parity_check_errors`, `parity_check_age_seconds`, `parity_check_duration_seconds` | | A última verificação. |
| `containers`, `containers_running`, `containers_autostart_stopped` | | Contentores Docker. |
| `container_running`, `container_autostart_stopped` | `container` | Por contentor; o segundo é 1 quando um contentor definido para iniciar com o array não está em execução. |
| `vms`, `vms_running` | | VMs. |
| `vm_state_info`, `vm_crashed` | `vm` (e `state`) | Por VM. |
| `notifications_unread` | `importance`: `alert`, `warning`, `info` | Notificações não lidas. |

As [regras incorporadas](../alerting/rules.md#unraid) que se aplicam:

- **Unraid array stopped**: o array não está iniciado há dez minutos (Warning).
- **Unraid disk disabled or missing**: o estado de um disco não é `DISK_OK`
  há cinco minutos (Warning). O array corre degradado.
- **Unraid disk read errors**: um disco reporta erros de leitura (Advisory).
- **Unraid disk too hot**: um disco de dados ou paridade acima de 55 °C
  durante quinze minutos (Advisory). Os discos de cache, frequentemente
  NVMe, são deixados de fora.
- **Unraid parity check found errors** (Advisory).
- **Unraid parity check overdue**: a última verificação tem mais de 40 dias
  (Advisory).
- **Unraid cache nearly full**: um disco de cache com mais de 90% de
  ocupação durante trinta minutos (Advisory).
- **Unraid container stopped**: um contentor definido para iniciar com o
  array não está em execução há dez minutos (Advisory).

## A página do monitor {#the-device-page}

O painel Array diz o que está mal numa frase, mostra a ocupação do array, a
última verificação de paridade, os contentores e VMs em execução, depois
lista cada disco — paridade primeiro, depois dados, depois cache — com o
seu estado, temperatura e ocupação, e os contentores que deveriam estar em
execução.

## Criar uma chave de API só de leitura para o DumbMonit {#create-a-read-only-api-key-for-dumbmonit}

1. No Unraid 7.2 ou posterior a API está incorporada. No Unraid 6.12 a 7.1, instale primeiro o plugin Unraid Connect a partir do separador Apps: traz a mesma API, e não é necessário nenhum início de sessão no Unraid Connect para uso local.

2. Crie uma chave de API com a função Viewer, que lê tudo e não pode alterar nada: em Settings > Management Access > API Keys, ou a partir do terminal do Unraid.

    ```
    unraid-api apikey --create --name dumbmonit --roles VIEWER --description "DumbMonit monitoring" --json
    ```

3. Copie a chave. No DumbMonit, introduza o endereço do servidor Unraid, por exemplo «tower.lan», e cole a chave como API key. Se a interface web for servida via HTTPS, escolha https como protocolo.

!!! warning
    O Unraid não reporta nenhuma temperatura para um disco que está parado, e o DumbMonit nunca acorda nenhum: um disco em suspensão simplesmente não tem temperatura até girar de novo.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Chave de API | Uma chave com a função Viewer, enviada como `x-api-key`. |

Endereço: o nome de anfitrião ou IP do servidor Unraid, `anfitrião:porta`,
ou um URL. Opções:

- **Protocol** (`http`) e **Port** (80): os da interface web. Com SSL
  definido como Strict em Settings > Management Access, use `https`, e o
  endereço `myunraid.net` se for esse o certificado.
- **Accept an unverifiable certificate**: para um certificado autoassinado.
- **Timeout per request** (15 s).

## Resolução de problemas {#troubleshooting}

«Unraid refused the API key (401)»: a chave está errada ou foi eliminada.

«Unraid accepted the API key but refused to answer (403)»: a chave não tem
função, ou uma função mais restrita do que Viewer.

Nenhum valor de contentor, VM ou paridade: essa consulta opcional falhou
(VMs desligadas, uma versão mais antiga da API sem o campo); o resto
continua a ser lido. O motivo está no registo do servidor ao nível de
depuração.
