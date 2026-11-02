# Monitores

Tudo o que o DumbMonit monitora, visto de quatro maneiras. No aplicativo é **Monitores** no menu da esquerda (`g d`), com quatro abas. **Adicionar um monitor** é o botão principal da aba Lista (`g n`).

| Aba | O que se vê | Ler |
|---|---|---|
| **Lista** | Cada monitor e o seu estado, filtros, pastas e ações em lote; um clique abre o monitor | [A página de monitores](../using/devices.md) |
| **Mapa** | O que depende de quê (`g m`) e os seus sites em um mapa-múndi | [Mapa de topologia](../using/topology.md), [Mapa-múndi](../using/world-map.md) |
| **Logs** | Uma única busca nos logs do servidor, dos agentes, do syslog e dos traps SNMP (`g l`, administradores) | [Logs](../using/logs.md) |
| **Segurança** | De onde vêm os ataques que o DumbMonit barrou (mapa de ameaças) e as vulnerabilidades conhecidas (CVE) dos softwares dos seus monitores (`g x`); a pontuação de segurança é ativada nas Configurações | [CVE e vulnerabilidades](../using/cves.md), [Pontuação de segurança](../using/security-score.md) |

O explorador de métricas (`Ctrl+K`, depois *Explore*) não é uma aba: percorre as métricas `dumbmonit_*` e testa uma consulta ([Explorador de métricas](../using/dashboards.md)).

## Adicionar monitores

- Um a um, a partir de **Adicionar um monitor**: [Adicionar seu primeiro monitor](../install/first-device.md).
- Varrendo uma rede: [Descoberta de rede](../using/discovery.md).
- A partir dos rótulos dos contêineres: [Monitores a partir de rótulos do Docker](../using/docker-labels.md).
- A partir de outra ferramenta (Uptime Kuma, Healthchecks, CSV): [Importar de outras ferramentas](../using/imports.md).
- A partir de arquivos guardados no Git: [Arquivos de configuração (GitOps)](../using/config-files.md).

## Integrações

Cada tipo de monitor tem a sua página, agrupada por categoria no menu da esquerda: servidores e agentes, virtualização, armazenamento e NAS, rede e firewalls, hardware, bancos de dados, servidores web, identidade, nuvem e SaaS, casa e mídia, aplicativos auto-hospedados, backup, e-mail, observabilidade e verificações de Internet. A tabela de [todas as integrações](../devices/index.md) lista todas com o que cada uma precisa.
