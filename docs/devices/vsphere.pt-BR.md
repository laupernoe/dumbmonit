# VMware vSphere {#vmware-vsphere}

Um vCenter Server e os hosts que ele gerencia, ou um host ESXi autônomo:
hosts que param de responder, ficam em modo de manutenção ou reportam uma
saúde vermelha, máquinas virtuais e suas VMware Tools, datastores enchendo
ou ficando inacessíveis, e os alarmes que o próprio vSphere disparou.

O DumbMonit lê o vSphere com um usuário dedicado que possui o papel embutido
**Read-only**. Ele nunca liga ou desliga uma VM, nunca entra em modo de
manutenção e nunca reconhece um alarme.

A falha que vale a pena observar: **um datastore que fica cheio**. Quando um
datastore thin provisioned atinge 100%, toda VM que precisa escrever nele é
pausada imediatamente. Os alarmes que o vCenter dispara também avisam isso,
mas só para quem tem o cliente aberto.

## Por que a Web Services API {#why-the-web-services-api}

O DumbMonit fala com a vSphere Web Services API (SOAP, `/sdk`), não com a
API REST (`/api/vcenter/*`): um host ESXi autônomo não tem API REST, e em um
vCenter a API REST não informa o modo de manutenção, a saúde geral
(`overallStatus`), o estado das VMware Tools ou os alarmes disparados. Uma
visão de todo o inventário é lida em uma única passagem, página por página,
e liberada ao final.

A sessão é mantida de uma sondagem para a próxima e reaberta apenas quando
expira: o vCenter não registra um novo login a cada minuto.

## O que ele observa {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_vsphere_`. Valores de saúde
seguem o `overallStatus` do vSphere: 0 verde, 1 amarelo, 2 vermelho, 3 cinza
(desconhecido).

| Métrica | O que é | Rótulos |
|---|---|---|
| `info` | valor 1 | `product` (`vcenter` ou `esxi`), `version`, `build` |
| `hosts` | hosts por estado de conexão: `connected`, `disconnected`, `notResponding` | `state` |
| `host_connection_state` | 0 conectado, 1 desconectado por um administrador, 2 não respondendo | `host` |
| `host_power_state` | 0 ligado, 1 em espera, 2 desligado, 3 desconhecido | `host` |
| `host_maintenance` | 1 em modo de manutenção | `host` |
| `host_status` | saúde geral | `host` |
| `host_cpu_percent`, `host_memory_percent`, `host_memory_bytes`, `host_uptime_seconds` | apenas hosts conectados | `host` |
| `vms` | VMs por estado de energia: `poweredOn`, `poweredOff`, `suspended` (templates excluídos) | `state` |
| `templates` | templates de VM | |
| `vm_power_state` | 0 ligada, 1 desligada, 2 suspensa; no máximo 1000 VMs | `vm` |
| `vm_status` | saúde geral | `vm` |
| `vm_tools_status` | apenas VMs ligadas: 0 atualizado, 1 versão antiga, 2 instalado mas não rodando, 3 não instalado | `vm` |
| `datastores` | datastores | |
| `datastore_capacity_bytes`, `datastore_free_bytes`, `datastore_used_percent` | | `datastore` |
| `datastore_accessible` | 1 acessível, 0 inacessível | `datastore` |
| `datastore_status` | saúde geral | `datastore` |
| `alarms` | alarmes disparados não reconhecidos, por cor: `red`, `yellow` | `status` |
| `alarms_acknowledged` | alarmes disparados já reconhecidos | |
| `alarm` | uma série por alarme não reconhecido, valor = seu código de saúde, no máximo 64 | `alarm` (seu nome), `entity`, `entity_type`, `status` |
| `scrape_errors` | chamadas que falharam na última sondagem | |

As [regras integradas](../alerting/rules.md#vmware-vsphere) que se aplicam:
vSphere host not responding, vSphere host health red, vSphere host left in
maintenance, vSphere datastore almost full, vSphere datastore inaccessible,
vSphere VM tools not running, vSphere red alarm, além de Device unreachable.

## A página do dispositivo {#the-device-page}

O painel acima dos gráficos lê o que a sondagem armazenou; abrir a página
nunca consulta o vSphere. Ele diz primeiro o que está errado (hosts não
respondendo, alarmes vermelhos, datastores quase cheios ou inacessíveis),
depois os hosts com sua carga, os datastores com seu nível de ocupação, e as
VMs por estado de energia.

## Como foi validado {#how-it-was-validated}

Contra o simulador govmomi `vcsim` 0.56.0, em modo vCenter e ESXi: login e
senha recusada, reutilização e expiração de sessão, paginação pelo
inventário, um host não respondendo, um host em modo de manutenção, uma VM
desligada e um template. O simulador não dispara nenhum alarme: a leitura
dos alarmes disparados é validada contra a estrutura documentada
`AlarmState` da Web Services API.

## Criar um usuário somente leitura no vSphere {#create-a-read-only-user-in-vsphere}

1. Em um vCenter: Administration → Single Sign On → Users and Groups →
   Users, domínio vsphere.local → Add. Nomeie o usuário como abaixo e dê a
   ele uma senha longa e aleatória.

    ```
    dumbmonit
    ```

2. Ainda no vCenter: Administration → Access Control → Global Permissions →
   Add. Escolha esse usuário, o papel Read-only, e marque Propagate to
   children. O Read-only vê todo host, VM, datastore e alarme, e não pode
   alterar nada.

3. Em um host ESXi autônomo, em vez disso: no host client, Manage →
   Security & users → Users → Add user, com o mesmo nome. Depois Host →
   Actions → Permissions → Add user: escolha-o, o papel Read-only, e marque
   Propagate to all children.

4. No DumbMonit, informe o endereço do vCenter ou do host, por exemplo
   "vcenter.lan" ou "esxi1.lan", com o nome de usuário como criado:
   dumbmonit@vsphere.local em um vCenter, dumbmonit em um host. Quando um
   vCenter gerencia os hosts, adicione apenas o vCenter: ele reporta todo
   host, VM e datastore.

5. O DumbMonit só lê, através da vSphere Web Services API (/sdk). Ele nunca
   liga ou desliga uma VM, nunca entra em modo de manutenção e nunca
   reconhece um alarme.

!!! warning
    Não reutilize a conta de domínio de single sign-on ou a conta embutida
    do host: o papel Read-only acima é tudo que o DumbMonit precisa. O
    vCenter e o ESXi usam um certificado autoassinado por padrão: ative
    "Aceitar um certificado não verificável" a menos que o host DumbMonit
    confie na autoridade certificadora deles.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Nome de usuário / senha | O usuário dedicado com o papel Read-only: `dumbmonit@vsphere.local` em um vCenter, `dumbmonit` em um host. |

Endereço: um nome de host ou IP (`vcenter.lan`), `host:porta`, ou uma URL
completa; um `/sdk` ou `/ui` final é aceito. Opções: porta (443),
verificação de certificado, tempo limite da requisição (20 s) e alarmes
disparados (lido).
