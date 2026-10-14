# Nutanix AHV {#nutanix-ahv}

Clusters Nutanix, lidos através da API REST v4 do Prism Central. Um Prism
Central cobre todos os clusters registados nele.

A cada minuto, o DumbMonit lê, com autenticação HTTP básica na porta 9440:

| O quê | Caminho |
|---|---|
| Clusters | `/api/clustermgmt/v4.0/config/clusters` |
| Anfitriões | `/api/clustermgmt/v4.0/config/hosts` |
| VM AHV | `/api/vmm/v4.0/ahv/config/vms` |
| Contentores de armazenamento | `/api/clustermgmt/v4.0/config/storage-containers` |
| Nível de ocupação de cada contentor | `/api/clustermgmt/v4.0/stats/storage-containers/{extId}` |
| Alertas por resolver | `/api/monitoring/v4.0/serviceability/alerts` |

As listas são lidas com 100 itens por página, até 1000 itens cada. O nível de
ocupação e os alertas são opcionais: se a função da conta os recusar, o resto
é recolhido na mesma. A Nutanix declarou obsoletas as APIs v1, v2.0 e v3; só a
v4 continua a evoluir, e exige o Prism Central pc.2024.3 ou posterior.

!!! warning "Validado apenas contra a documentação do fornecedor — ainda não testado num sistema real"

    Esta integração foi construída a partir das especificações da API v4 da
    Nutanix (caminhos e campos tal como os definem os clientes gerados
    oficiais) e testada contra respostas gravadas, não contra um Prism Central
    em funcionamento. Diga-nos o que falha.

## O que monitoriza {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_nutanix_`.

| Métrica | Etiquetas | Significado |
|---|---|---|
| `clusters` | | Clusters, sem o próprio Prism Central. |
| `cluster_info` | `cluster`, `version` | Valor 1: a versão do AOS de cada cluster. |
| `cluster_available` | `cluster` | 1 quando o Prism Central alcança o cluster. |
| `hosts` | | Anfitriões de todos os clusters. |
| `host_info` | `host`, `cluster`, `hypervisor` | Valor 1: o hipervisor que cada anfitrião executa. |
| `host_cpu_cores`, `host_memory_bytes` | `host`, `cluster` | Capacidade. |
| `host_degraded` | `host`, `cluster` | 1 quando a Nutanix marca o anfitrião como degradado. |
| `host_maintenance` | `host`, `cluster` | 1 quando o anfitrião está a entrar em modo de manutenção ou está nele. |
| `vms`, `vms_running` | | VM AHV, e quantas estão ligadas. |
| `vm_power_state_info` | `vm`, `cluster`, `state` | Valor 1: `on`, `off`, `paused` ou `undetermined`. |
| `vm_running` | `vm`, `cluster` | 1 quando a VM está ligada. |
| `vm_vcpus`, `vm_memory_bytes` | `vm`, `cluster` | Sockets × núcleos × threads, e memória configurada. |
| `storage_containers` | | Contentores de armazenamento, sem os internos da Nutanix. |
| `storage_container_capacity_bytes`, `storage_container_used_bytes`, `storage_container_used_percent` | `container`, `cluster` | Nível de ocupação, a partir das últimas estatísticas dos últimos trinta minutos. |
| `alerts_unresolved`, `alerts_critical`, `alerts_warning` | | Alertas por resolver, todos e por severidade. |

As [regras incorporadas](../alerting/rules.md#nutanix) que se aplicam:

- **Nutanix host degraded**: um anfitrião está marcado como degradado há cinco
  minutos (Warning).
- **Nutanix storage container almost full**: um contentor está 90% cheio
  durante quinze minutos (Advisory).
- **Nutanix critical alerts**: o Prism Central reporta pelo menos um alerta
  crítico por resolver há cinco minutos (Warning).

## Criar um utilizador só de leitura no Prism Central {#create-a-read-only-user-in-prism-central}

1. No Prism Central, abra Admin Center > IAM > Identities e adicione um utilizador local com uma senha longa e aleatória, com o seguinte nome.

    ```
    dumbmonit
    ```

2. Em Admin Center > IAM > Authorization Policies, crie uma política com a função integrada Prism Viewer, para todas as entidades, e atribua-a a esse utilizador. O Prism Viewer lê clusters, anfitriões, VM, contentores de armazenamento e alertas, e não altera nada.

3. No DumbMonit, introduza o endereço do Prism Central, por exemplo «prism-central.lan», com esse nome de utilizador e senha. Um Prism Central reporta todos os clusters registados nele.

4. O DumbMonit só lê a API v4 (clustermgmt, vmm e monitoring, versão 4.0): nunca liga nem desliga uma VM, nunca resolve um alerta e nunca altera um contentor de armazenamento.

!!! warning
    A API v4 é servida pelo Prism Central pc.2024.3 ou posterior; um cluster gerido apenas pelo Prism Element, ou por um Prism Central mais antigo, não é suportado. O Prism Central traz um certificado autoassinado: marque Accept an unverifiable certificate a menos que o anfitrião do DumbMonit confie na sua autoridade de certificação.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Nome de utilizador e senha | O utilizador local `dumbmonit` com a função Prism Viewer. |

Endereço: o nome do anfitrião ou o IP do Prism Central, `host:port`, ou um
URL. Opções:

- **Porta** (9440).
- **Aceitar um certificado não verificável**: o próprio certificado do Prism
  Central é autoassinado.
- **Tempo limite por pedido** (15 s).

## Resolução de problemas {#troubleshooting}

«Nutanix Prism Central refused the credentials … (401)»: nome de utilizador ou
senha errados.

«Nutanix Prism Central has no /api/clustermgmt/v4.0/…»: o endereço é o de um
Prism Element, ou de um Prism Central anterior ao pc.2024.3.

Sem `storage_container_used_percent`: a função não permite ler as
estatísticas; os contentores são contados na mesma, com a sua capacidade
configurada.
