# Unraid {#unraid}

Um servidor Unraid através de sua API GraphQL: um array que não está
iniciado, discos desativados, faltando ou reportando erros de leitura,
verificações de paridade que encontraram erros ou não rodam há semanas,
pools de cache enchendo, contêineres que deveriam estar rodando e não estão,
e o estado das VMs.

O DumbMonit envia uma chave de API com o papel Viewer no cabeçalho
`x-api-key` para `/graphql`. O papel Viewer lê tudo e não pode alterar nada.
A API é embutida no Unraid a partir da 7.2; na 6.12 a 7.1 ela vem com o
plugin Unraid Connect.

A cada minuto o DumbMonit envia uma consulta obrigatória (versão, array,
discos de paridade, dados e cache) e quatro opcionais, cada uma por conta
própria para que um campo que uma versão mais antiga da API não tem, ou VMs
desligadas, só deixem de fora aquela parte: a verificação de paridade, os
contêineres Docker, as VMs e as notificações não lidas.

!!! warning "Validado apenas contra a documentação do fornecedor — ainda não testado em um sistema real"

    Esta integração foi construída a partir da documentação da API do
    Unraid e do esquema GraphQL publicado da API do Unraid (nomes de
    campos, enumerações, tamanhos em kilobytes), não contra um servidor em
    execução. Toda consulta é uma leitura. Avise-nos o que não funcionar.

## O que ele observa {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_unraid_`.

| Métrica | Rótulos | Significado |
|---|---|---|
| `version_info` | `version` | Valor 1: a versão do Unraid. |
| `array_started` | | 1 quando o array está iniciado. |
| `array_state_info` | `state` | Valor 1: `started`, `stopped`, `new_array`, `recon_disk`, `disable_disk`… |
| `array_size_bytes`, `array_used_bytes`, `array_free_bytes`, `array_used_percent` | | Capacidade do array. |
| `disk_ok` | `disk`, `role` (`parity`, `data`, `cache`) | 1 quando o status do disco é `DISK_OK`. Slots vazios são deixados de fora. |
| `disk_status_info` | `disk`, `role`, `status` | Valor 1: o status do Unraid (`disk_ok`, `disk_dsbl`, `disk_np_missing`, `disk_invalid`…). |
| `disk_errors` | `disk`, `role` | Erros de leitura desde que o array iniciou (a coluna Errors). |
| `disk_temperature_celsius` | `disk`, `role` | Apenas enquanto o disco gira: o Unraid não reporta nenhuma para um disco dormindo. |
| `disk_spinning` | `disk`, `role` | 1 quando o disco está girando. |
| `disk_size_bytes`, `disk_fs_size_bytes`, `disk_used_percent` | `disk`, `role` | Tamanho, tamanho do sistema de arquivos e ocupação, para discos com um sistema de arquivos. |
| `disks`, `disks_problem`, `disks_with_errors` | | Contagens. |
| `parity_check_status_info` | `status` | Valor 1: `completed`, `running`, `paused`, `cancelled`, `failed`, `never_run`. |
| `parity_check_running`, `parity_check_progress_percent` | | Uma verificação em andamento. |
| `parity_check_ok` | | Após uma verificação concluída: 1 sem erros de sincronização, 0 com erros ou quando falhou. Uma verificação cancelada não prova nada e não define nada. |
| `parity_check_errors`, `parity_check_age_seconds`, `parity_check_duration_seconds` | | A última verificação. |
| `containers`, `containers_running`, `containers_autostart_stopped` | | Contêineres Docker. |
| `container_running`, `container_autostart_stopped` | `container` | Por contêiner; o segundo é 1 quando um contêiner configurado para iniciar com o array não está rodando. |
| `vms`, `vms_running` | | VMs. |
| `vm_state_info`, `vm_crashed` | `vm` (e `state`) | Por VM. |
| `notifications_unread` | `importance`: `alert`, `warning`, `info` | Notificações não lidas. |

As [regras integradas](../alerting/rules.md#unraid) que se aplicam:

- **Unraid array stopped**: o array não está iniciado por dez minutos
  (Warning).
- **Unraid disk disabled or missing**: o status de um disco não é
  `DISK_OK` por cinco minutos (Warning). O array roda degradado.
- **Unraid disk read errors**: um disco reporta erros de leitura
  (Advisory).
- **Unraid disk too hot**: um disco de dados ou paridade acima de 55 °C
  por quinze minutos (Advisory). Discos de cache, frequentemente NVMe, são
  deixados de fora.
- **Unraid parity check found errors** (Advisory).
- **Unraid parity check overdue**: a última verificação tem mais de 40 dias
  (Advisory).
- **Unraid cache nearly full**: um disco de cache com mais de 90% de
  ocupação por trinta minutos (Advisory).
- **Unraid container stopped**: um contêiner configurado para iniciar com o
  array não está rodando há dez minutos (Advisory).

## A página do dispositivo {#the-device-page}

O painel Array diz o que está errado em uma frase, mostra a ocupação do
array, a última verificação de paridade, os contêineres e VMs em execução,
depois lista cada disco — paridade primeiro, depois dados, depois cache —
com seu status, temperatura e ocupação, e os contêineres que deveriam estar
rodando.

## Criar uma chave de API somente leitura para o DumbMonit {#create-a-read-only-api-key-for-dumbmonit}

1. No Unraid 7.2 ou posterior a API é embutida. No Unraid 6.12 a 7.1,
   instale o plugin Unraid Connect da aba Apps primeiro: ele traz a mesma
   API, e nenhum login no Unraid Connect é necessário para uso local.

2. Crie uma chave de API com o papel Viewer, que lê tudo e não pode alterar
   nada: em Settings > Management Access > API Keys, ou no terminal do
   Unraid.

    ```
    unraid-api apikey --create --name dumbmonit --roles VIEWER --description "DumbMonit monitoring" --json
    ```

3. Copie a chave. No DumbMonit, informe o endereço do servidor Unraid, por
   exemplo "tower.lan", e cole a chave como API key. Se a interface web é
   servida via HTTPS, escolha https como protocolo.

!!! warning
    O Unraid não reporta temperatura para um disco que está parado, e o
    DumbMonit nunca desperta um: um disco dormindo simplesmente não tem
    temperatura até girar de novo.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Chave de API | Uma chave com o papel Viewer, enviada como `x-api-key`. |

Endereço: o nome de host ou IP do servidor Unraid, `host:porta`, ou uma URL.
Opções:

- **Protocolo** (`http`) e **Porta** (80): os da interface web. Com SSL
  definido como Strict em Settings > Management Access, use `https`, e o
  endereço `myunraid.net` se for esse o certificado.
- **Aceitar um certificado não verificável**: para um certificado
  autoassinado.
- **Tempo limite por requisição** (15 s).

## Solução de problemas {#troubleshooting}

"Unraid refused the API key (401)": a chave está errada ou foi excluída.

"Unraid accepted the API key but refused to answer (403)": a chave não tem
papel, ou um papel mais restrito que Viewer.

Nenhum número de contêiner, VM ou paridade: essa consulta opcional falhou
(VMs desligadas, uma versão mais antiga da API sem o campo); o resto ainda é
lido. O motivo está no log do servidor em nível de debug.
