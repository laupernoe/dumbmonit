# Nutanix AHV {#nutanix-ahv}

Clusters Nutanix, lidos pela API REST v4 do Prism Central. Um Prism Central
cobre todos os clusters registrados nele.

A cada minuto, o DumbMonit lê, com autenticação HTTP básica na porta 9440:

| O quê | Caminho |
|---|---|
| Clusters | `/api/clustermgmt/v4.0/config/clusters` |
| Hosts | `/api/clustermgmt/v4.0/config/hosts` |
| VMs AHV | `/api/vmm/v4.0/ahv/config/vms` |
| Contêineres de armazenamento | `/api/clustermgmt/v4.0/config/storage-containers` |
| Nível de ocupação de cada contêiner | `/api/clustermgmt/v4.0/stats/storage-containers/{extId}` |
| Alertas não resolvidos | `/api/monitoring/v4.0/serviceability/alerts` |

As listas são lidas com 100 itens por página, até 1.000 itens cada. O nível de
ocupação e os alertas são opcionais: se o papel da conta os recusar, o resto é
coletado mesmo assim. A Nutanix declarou obsoletas as APIs v1, v2.0 e v3; só a
v4 continua evoluindo, e ela exige o Prism Central pc.2024.3 ou posterior.

!!! warning "Validado apenas contra a documentação do fornecedor — ainda não testado em um sistema real"

    Esta integração foi construída a partir das especificações da API v4 da
    Nutanix (caminhos e campos como os definem os clientes gerados oficiais) e
    testada contra respostas gravadas, não contra um Prism Central em
    execução. Avise-nos o que não funcionar.

## O que ele observa {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_nutanix_`.

| Métrica | Etiquetas | Significado |
|---|---|---|
| `clusters` | | Clusters, sem o próprio Prism Central. |
| `cluster_info` | `cluster`, `version` | Valor 1: a versão do AOS de cada cluster. |
| `cluster_available` | `cluster` | 1 quando o Prism Central alcança o cluster. |
| `hosts` | | Hosts de todos os clusters. |
| `host_info` | `host`, `cluster`, `hypervisor` | Valor 1: o hypervisor que cada host executa. |
| `host_cpu_cores`, `host_memory_bytes` | `host`, `cluster` | Capacidade. |
| `host_degraded` | `host`, `cluster` | 1 quando a Nutanix marca o host como degradado. |
| `host_maintenance` | `host`, `cluster` | 1 quando o host está entrando em modo de manutenção ou está nele. |
| `vms`, `vms_running` | | VMs AHV, e quantas estão ligadas. |
| `vm_power_state_info` | `vm`, `cluster`, `state` | Valor 1: `on`, `off`, `paused` ou `undetermined`. |
| `vm_running` | `vm`, `cluster` | 1 quando a VM está ligada. |
| `vm_vcpus`, `vm_memory_bytes` | `vm`, `cluster` | Sockets × núcleos × threads, e memória configurada. |
| `storage_containers` | | Contêineres de armazenamento, sem os internos da Nutanix. |
| `storage_container_capacity_bytes`, `storage_container_used_bytes`, `storage_container_used_percent` | `container`, `cluster` | Nível de ocupação, a partir das últimas estatísticas dos últimos trinta minutos. |
| `alerts_unresolved`, `alerts_critical`, `alerts_warning` | | Alertas não resolvidos, todos e por severidade. |

As [regras integradas](../alerting/rules.md#nutanix) que se aplicam:

- **Nutanix host degraded**: um host está marcado como degradado há cinco
  minutos (Alerta).
- **Nutanix storage container almost full**: um contêiner está 90% cheio por
  quinze minutos (Aviso).
- **Nutanix critical alerts**: o Prism Central reporta pelo menos um alerta
  crítico não resolvido há cinco minutos (Alerta).

## Criar um usuário somente leitura no Prism Central {#create-a-read-only-user-in-prism-central}

1. No Prism Central, abra Admin Center > IAM > Identities e adicione um usuário local com uma senha longa e aleatória, com o seguinte nome.

    ```
    dumbmonit
    ```

2. Em Admin Center > IAM > Authorization Policies, crie uma política com o papel integrado Prism Viewer, para todas as entidades, e atribua-a a esse usuário. O Prism Viewer lê clusters, hosts, VMs, contêineres de armazenamento e alertas, e não altera nada.

3. No DumbMonit, informe o endereço do Prism Central, por exemplo "prism-central.lan", com esse nome de usuário e senha. Um Prism Central reporta todos os clusters registrados nele.

4. O DumbMonit só lê a API v4 (clustermgmt, vmm e monitoring, versão 4.0): nunca liga nem desliga uma VM, nunca resolve um alerta e nunca altera um contêiner de armazenamento.

!!! warning
    A API v4 é servida pelo Prism Central pc.2024.3 ou posterior; um cluster gerenciado apenas pelo Prism Element, ou por um Prism Central mais antigo, não é suportado. O Prism Central vem com um certificado autoassinado: marque Accept an unverifiable certificate a menos que o host do DumbMonit confie na sua autoridade certificadora.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Nome de usuário e senha | O usuário local `dumbmonit` com o papel Prism Viewer. |

Endereço: o nome do host ou o IP do Prism Central, `host:port`, ou uma URL.
Opções:

- **Porta** (9440).
- **Aceitar um certificado não verificável**: o certificado próprio do Prism
  Central é autoassinado.
- **Timeout por requisição** (15 s).

## Solução de problemas {#troubleshooting}

"Nutanix Prism Central refused the credentials … (401)": nome de usuário ou
senha errados.

"Nutanix Prism Central has no /api/clustermgmt/v4.0/…": o endereço é o de um
Prism Element, ou de um Prism Central anterior ao pc.2024.3.

Sem `storage_container_used_percent`: o papel não permite ler as estatísticas;
os contêineres são contados mesmo assim, com a capacidade configurada.
