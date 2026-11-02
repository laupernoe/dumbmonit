# Monitores

Tudo o que o DumbMonit vigia, visto de quatro maneiras. Na aplicação é **Monitores** no menu da esquerda (`g d`), com quatro separadores. **Adicionar um monitor** é o botão principal do separador Lista (`g n`).

| Separador | O que se vê | Ler |
|---|---|---|
| **Lista** | Cada monitor e o seu estado, filtros, pastas e ações em lote; um clique abre o monitor | [A página de monitores](../using/devices.md) |
| **Mapa** | O que depende de quê (`g m`) e os seus sites num mapa-múndi | [Mapa de topologia](../using/topology.md), [Mapa-múndi](../using/world-map.md) |
| **Registos** | Uma só pesquisa nos registos do servidor, dos agentes, do syslog e dos traps SNMP (`g l`, administradores) | [Registos](../using/logs.md) |
| **Segurança** | De onde vêm os ataques que o DumbMonit travou (mapa de ameaças) e as vulnerabilidades conhecidas (CVE) do software dos seus monitores (`g x`); a pontuação de segurança ativa-se nas Definições | [CVE e vulnerabilidades](../using/cves.md), [Pontuação de segurança](../using/security-score.md) |

O explorador de métricas (`Ctrl+K`, depois *Explore*) não é um separador: percorre as métricas `dumbmonit_*` e experimenta uma consulta ([Explorador de métricas](../using/dashboards.md)).

## Adicionar monitores

- Um a um, a partir de **Adicionar um monitor**: [Adicionar o primeiro monitor](../install/first-device.md).
- Analisando uma rede: [Descoberta de rede](../using/discovery.md).
- A partir das etiquetas dos contentores: [Monitores a partir de etiquetas Docker](../using/docker-labels.md).
- A partir de outra ferramenta (Uptime Kuma, Healthchecks, CSV): [Importar de outras ferramentas](../using/imports.md).
- A partir de ficheiros guardados no Git: [Ficheiros de configuração (GitOps)](../using/config-files.md).

## Integrações

Cada tipo de monitor tem a sua página, agrupada por categoria no menu da esquerda: servidores e agentes, virtualização, armazenamento e NAS, rede e firewalls, hardware, bases de dados, servidores web, identidade, cloud e SaaS, casa e multimédia, aplicações auto-alojadas, cópias de segurança, e-mail, observabilidade e verificações de Internet. A tabela de [todas as integrações](../devices/index.md) lista-as com o que cada uma precisa.
