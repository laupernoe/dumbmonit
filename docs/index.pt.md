# DumbMonit {#dumbmonit}

Monitorização simples, do homelab à pequena empresa. Um contentor, um endereço IP
para escrever, gráficos úteis e alertas em menos de um minuto.

!!! warning "Trabalho em curso"
    O DumbMonit está em desenvolvimento ativo; a imagem atual
    (`ghcr.io/laupernoe/dumbmonit:latest`) é uma alfa para os primeiros testadores.
    Conte com arestas por limar e alterações incompatíveis até à primeira versão.

O DumbMonit lê a rede como um **boletim meteorológico**: a página inicial diz
o estado do céu numa frase («Céu limpo.» ou «2 avisos, 1 inacessível.») e
lista o que precisa de si, antes de mais nada. As severidades seguem a escala
meteorológica (info → aviso → alerta), as previsões são previsões do tempo e as
janelas de manutenção são agendadas.

![A página de visão geral: a frase do boletim, a lista «Precisa de si» e as previsões](assets/screenshots/overview-light.png){ loading=lazy }

<div class="dm-links" markdown>
<a href="install/docker/">Instalar<small>Docker Compose, primeiro arranque, cópias de segurança</small></a>
<a href="install/first-device/">Adicionar o primeiro dispositivo<small>SNMP, análise de rede, o que acontece a seguir</small></a>
<a href="alerting/">Alertas<small>Regras integradas, silenciosas por construção</small></a>
</div>

## O que monitoriza {#what-it-watches}

| Origem | O que obtém |
|---|---|
| [SNMP v1 / v2c / v3](devices/snmp.md) | Comutadores, routers, NAS, UPS, impressoras. Cinco perfis acompanham o produto e são aplicados automaticamente a partir do `sysObjectID` do dispositivo. Uma análise de rede adiciona de uma só vez tudo o que responder. |
| [Proxmox VE](devices/proxmox.md) | Nós, máquinas virtuais e contentores, armazenamentos, quórum do cluster e idade da última cópia de segurança bem-sucedida por máquina. |
| [Proxmox Backup Server](devices/pbs.md) | Utilização do datastore e previsão de enchimento, deduplicação, idade e verificação do último snapshot de cada máquina, tarefas falhadas, recolha de lixo. |
| [Synology DSM](devices/synology.md) | Volumes, discos e o seu estado SMART, temperatura, carga, através da API web do NAS. |
| [Agente Linux, macOS, FreeBSD e Windows](devices/agent.md) | CPU, memória, discos, rede, serviços, contentores e tempo de atividade de máquinas que não falam SNMP, além de temperaturas, estado dos discos (SMART) e pools ZFS quando a máquina os expõe. Um comando para instalar, ou uma imagem Docker; em [modo relay](install/remote-site.md) sonda os dispositivos de um local remoto apenas com ligações de saída. |
| [Serviços](devices/services.md) | HTTP(S), porta TCP, DNS, ping e validade de certificados TLS, ao estilo Uptime Kuma, com barra de histórico e percentagem de disponibilidade. |
| [Pacotes de integração](packs/index.md) | Um tipo de dispositivo declarado num único ficheiro YAML — uma API HTTP ou uma página Prometheus `/metrics` no dispositivo — com as suas próprias regras de alerta, instalado sem uma nova versão do servidor. |

## Fale com ele {#talk-to-it}

- Um [assistente via MCP](using/assistant.md) — Claude Code, Claude Desktop,
  ChatGPT, VS Code, Cursor — com 27 ferramentas: um token `read` apenas consulta
  (estado, dispositivos, alertas, métricas, agentes, contentores…), um token
  `write` também pode agir (silenciar, reconhecer, adicionar um dispositivo,
  reiniciar um contentor, publicar um incidente numa página de estado). Os
  segredos nunca são devolvidos.
- A [API HTTP](reference/api.md) por trás de tudo o que a UI faz, descrita em
  `/api/openapi.json` (OpenAPI 3.1). Os tokens (`dmt_…`) têm âmbito de leitura ou
  escrita, podem expirar, ser restritos a uma lista de redes e têm limitação de
  taxa.

## O que corre {#what-runs}

| Contentor | Função | Consumo |
|---|---|---|
| `dumbmonit` | Recolha, API, alertas, UI web e o VictoriaMetrics integrado para séries temporais | ~40 MB de RAM + o orçamento do VictoriaMetrics (256 MB por omissão) |

Um contentor, um volume: o servidor inicia o VictoriaMetrics a partir da mesma
imagem, e a configuração e o estado vivem numa base de dados SQLite integrada.
Não há contentor de base de dados. Pode usar-se um VictoriaMetrics externo
(`DUMBMONIT_VM_URL`).

!!! note "Sobre o nome"
    O DumbMonit chamava-se EzyMonit até setembro de 2026. Os comandos, as variáveis
    de ambiente, os nomes de imagens e os caminhos foram renomeados com ele; as
    antigas variáveis `EZYMONIT_*` e os tokens de agente `ezym_` continuam a ser
    aceites. Veja [Atualização](install/docker.md#upgrading).

O DumbMonit é 100% código aberto sob a licença Apache 2.0, dependências
incluídas: nenhuma funcionalidade é reservada a uma edição paga.
