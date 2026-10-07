# DumbMonit

Monitoramento simples, do homelab à pequena empresa. Um contêiner, um endereço IP
para digitar, gráficos e alertas úteis em menos de um minuto.

!!! warning "Em desenvolvimento"
    O DumbMonit está em desenvolvimento ativo; a imagem atual
    (`ghcr.io/laupernoe/dumbmonit:latest`) é uma alfa para os primeiros testadores. Espere
    arestas por aparar e mudanças incompatíveis até a primeira versão.

O DumbMonit lê a rede como um **boletim meteorológico**: a página inicial diz
como está o céu em uma frase ("Céu limpo." ou "2 avisos, 1 inacessível.") e
lista o que precisa de você, antes de qualquer outra coisa. As severidades seguem
a escala meteorológica (info → aviso → alerta), as previsões são prognósticos, e as
janelas de manutenção são programadas.

![A página de visão geral: a frase do boletim, a lista "Precisa de você" e as previsões](assets/screenshots/overview-light.png){ loading=lazy }

<div class="dm-links" markdown>
<a href="install/docker/">Instalar<small>Docker Compose, primeira inicialização, backups</small></a>
<a href="install/first-device/">Adicione seu primeiro dispositivo<small>SNMP, varredura de rede, o que acontece depois</small></a>
<a href="alerting/">Alertas<small>Regras integradas, silenciosas por construção</small></a>
</div>

## O que ele monitora {#what-it-watches}

| Fonte | O que você obtém |
|---|---|
| [SNMP v1 / v2c / v3](devices/snmp.md) | Switches, roteadores, NAS, nobreaks, impressoras. Cinco perfis acompanham o produto e são aplicados automaticamente a partir do `sysObjectID` do dispositivo. Uma varredura de rede adiciona de uma só vez tudo o que responder. |
| [Proxmox VE](devices/proxmox.md) | Nós, máquinas virtuais e contêineres, armazenamentos, quórum do cluster e a idade do último backup bem-sucedido de cada máquina. |
| [Proxmox Backup Server](devices/pbs.md) | Uso do datastore e previsão de esgotamento, deduplicação, idade e verificação do último snapshot de cada máquina, tarefas com falha, coleta de lixo. |
| [Synology DSM](devices/synology.md) | Volumes, discos e sua saúde SMART, temperatura, carga, por meio da API web do NAS. |
| [Agente Linux, macOS, FreeBSD e Windows](devices/agent.md) | CPU, memória, discos, rede, serviços, contêineres e tempo de atividade de máquinas que não falam SNMP, além de temperaturas, saúde dos discos (SMART) e pools ZFS quando a máquina os expõe. Um comando para instalar, ou uma imagem Docker; no [modo relay](install/remote-site.md) ele sonda os dispositivos de um site remoto usando apenas conexões de saída. |
| [Serviços](devices/services.md) | HTTP(S), porta TCP, DNS, ping e expiração de certificado TLS, no estilo Uptime Kuma, com barra de histórico e porcentagem de disponibilidade. |
| [Pacotes de integração](packs/index.md) | Um tipo de dispositivo declarado em um único arquivo YAML — uma API HTTP ou uma página Prometheus `/metrics` no dispositivo — com suas próprias regras de alerta, instalado sem uma nova versão do servidor. |

## Converse com ele {#talk-to-it}

- Um [assistente via MCP](using/assistant.md) — Claude Code, Claude Desktop,
  ChatGPT, VS Code, Cursor — com 27 ferramentas: um token `read` apenas observa (status,
  dispositivos, alertas, métricas, agentes, contêineres…), um token `write` também pode agir
  (silenciar, reconhecer, adicionar um dispositivo, reiniciar um contêiner, publicar um
  incidente na página de status). Os segredos nunca são retornados.
- A [API HTTP](reference/api.md) por trás de tudo o que a interface faz, descrita em
  `/api/openapi.json` (OpenAPI 3.1). Os tokens (`dmt_…`) têm escopo de leitura ou
  escrita, podem expirar, ser restritos a uma lista de redes, e têm
  limite de requisições.

## O que está em execução {#what-runs}

| Contêiner | Função | Consumo |
|---|---|---|
| `dumbmonit` | Coleta, API, alertas, interface web e o VictoriaMetrics embutido para séries temporais | ~40 MB de RAM + o orçamento do VictoriaMetrics (256 MB por padrão) |

Um contêiner, um volume: o servidor inicia o VictoriaMetrics a partir da mesma
imagem, e a configuração e o estado ficam em um banco SQLite embutido. Não há
contêiner de banco de dados. Um VictoriaMetrics externo pode ser usado no lugar
(`DUMBMONIT_VM_URL`).

!!! note "Sobre o nome"
    O DumbMonit se chamava EzyMonit até setembro de 2026. Comandos, variáveis de
    ambiente, nomes de imagem e caminhos foram renomeados junto; as antigas
    variáveis `EZYMONIT_*` e os tokens de agente `ezym_` ainda são aceitos. Veja
    [Atualização](install/docker.md#upgrading).

O DumbMonit é 100% código aberto sob a licença Apache 2.0, dependências
incluídas: nenhum recurso é reservado a uma edição paga.
