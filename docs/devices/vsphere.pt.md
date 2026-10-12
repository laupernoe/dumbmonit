# VMware vSphere {#vmware-vsphere}

Um vCenter Server e os anfitriões que gere, ou um anfitrião ESXi autónomo:
anfitriões que deixam de responder, ficam em modo de manutenção ou reportam
uma saúde vermelha, máquinas virtuais e as suas VMware Tools, datastores a
enchê-se ou a ficarem inacessíveis, e os alarmes que o próprio vSphere
levantou.

O DumbMonit lê o vSphere com um utilizador dedicado que detém a função
incorporada **Read-only**. Nunca liga ou desliga uma VM, nunca entra em
modo de manutenção e nunca confirma um alarme.

A falha a vigiar: **um datastore que se enche**. Quando um datastore
thin provisioned atinge 100%, toda VM que precisa de escrever nele é
pausada de imediato. Os alarmes que o vCenter levanta também o dizem, mas
só a quem tem o cliente aberto.

## Por que a Web Services API {#why-the-web-services-api}

O DumbMonit fala com a API Web Services do vSphere (SOAP, `/sdk`), não com a
API REST (`/api/vcenter/*`): um anfitrião ESXi autónomo não tem API REST, e
num vCenter a API REST não diz o modo de manutenção, a saúde geral
(`overallStatus`), o estado das VMware Tools nem os alarmes disparados. Uma
vista de todo o inventário é lida numa única passagem, página a página, e
libertada ao fim.

A sessão é mantida de uma verificação para a outra e reaberta só quando
expira: o vCenter não regista um novo início de sessão a cada minuto.

## O que monitoriza {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_vsphere_`. Os valores de saúde
seguem o `overallStatus` do vSphere: 0 verde, 1 amarelo, 2 vermelho, 3 cinza
(desconhecido).

| Métrica | O quê | Etiquetas |
|---|---|---|
| `info` | valor 1 | `product` (`vcenter` ou `esxi`), `version`, `build` |
| `hosts` | anfitriões por estado de ligação: `connected`, `disconnected`, `notResponding` | `state` |
| `host_connection_state` | 0 ligado, 1 desligado por um administrador, 2 sem resposta | `host` |
| `host_power_state` | 0 ligado, 1 em espera, 2 desligado, 3 desconhecido | `host` |
| `host_maintenance` | 1 em modo de manutenção | `host` |
| `host_status` | saúde geral | `host` |
| `host_cpu_percent`, `host_memory_percent`, `host_memory_bytes`, `host_uptime_seconds` | só anfitriões ligados | `host` |
| `vms` | VMs por estado de alimentação: `poweredOn`, `poweredOff`, `suspended` (modelos excluídos) | `state` |
| `templates` | modelos de VM | |
| `vm_power_state` | 0 ligada, 1 desligada, 2 suspensa; no máximo 1000 VMs | `vm` |
| `vm_status` | saúde geral | `vm` |
| `vm_tools_status` | só VMs ligadas: 0 atualizadas, 1 versão antiga, 2 instaladas mas não em execução, 3 não instaladas | `vm` |
| `datastores` | datastores | |
| `datastore_capacity_bytes`, `datastore_free_bytes`, `datastore_used_percent` | | `datastore` |
| `datastore_accessible` | 1 acessível, 0 inacessível | `datastore` |
| `datastore_status` | saúde geral | `datastore` |
| `alarms` | alarmes disparados não confirmados, por cor: `red`, `yellow` | `status` |
| `alarms_acknowledged` | alarmes disparados já confirmados | |
| `alarm` | uma série por alarme não confirmado, valor = o seu código de saúde, no máximo 64 | `alarm` (o seu nome), `entity`, `entity_type`, `status` |
| `scrape_errors` | chamadas que falharam na última verificação | |

As [regras incorporadas](../alerting/rules.md#vmware-vsphere) que se
aplicam: vSphere host not responding, vSphere host health red, vSphere host
left in maintenance, vSphere datastore almost full, vSphere datastore
inaccessible, vSphere VM tools not running, vSphere red alarm, mais Device
unreachable.

## A página do dispositivo {#the-device-page}

O painel acima dos gráficos lê o que a verificação guardou; abrir a página
nunca consulta o vSphere. Diz primeiro o que está mal (anfitriões sem
resposta, alarmes vermelhos, datastores quase cheios ou inacessíveis),
depois os anfitriões com a sua carga, os datastores com o seu nível de
ocupação, e as VMs por estado de alimentação.

## Como foi validado {#how-it-was-validated}

Contra o simulador govmomi `vcsim` 0.56.0, em modo vCenter e ESXi: início de
sessão e senha recusada, reutilização e expiração de sessão, paginação pelo
inventário, um anfitrião sem resposta, um anfitrião em modo de manutenção,
uma VM desligada e um modelo. O simulador não levanta nenhum alarme: a
leitura dos alarmes disparados é validada contra a estrutura `AlarmState`
documentada da Web Services API.

## Criar um utilizador só de leitura no vSphere {#create-a-read-only-user-in-vsphere}

1. Num vCenter: Administration → Single Sign On → Users and Groups → Users, domínio vsphere.local → Add. Dê o seguinte nome ao utilizador e uma senha longa e aleatória.

    ```
    dumbmonit
    ```

2. Ainda no vCenter: Administration → Access Control → Global Permissions → Add. Escolha esse utilizador, a função Read-only, e marque Propagate to children. Read-only vê todos os anfitriões, VMs, datastores e alarmes, e não pode alterar nada.

3. Num anfitrião ESXi autónomo em vez disso: no cliente do anfitrião, Manage → Security & users → Users → Add user, com o mesmo nome. Depois Host → Actions → Permissions → Add user: escolha-o, a função Read-only, e marque Propagate to all children.

4. No DumbMonit, introduza o endereço do vCenter ou do anfitrião, por exemplo «vcenter.lan» ou «esxi1.lan», com o nome de utilizador criado: dumbmonit@vsphere.local num vCenter, dumbmonit num anfitrião. Quando um vCenter gere os anfitriões, adicione só o vCenter: reporta todos os anfitriões, VMs e datastores.

5. O DumbMonit só lê, através da API Web Services do vSphere (/sdk). Nunca liga ou desliga uma VM, nunca entra em modo de manutenção e nunca confirma um alarme.

!!! warning
    Não reutilize a conta de domínio do single sign-on nem a conta incorporada do anfitrião: a função Read-only acima é tudo o que o DumbMonit precisa. O vCenter e o ESXi usam um certificado autoassinado por predefinição: ative «Accept an unverifiable certificate» a menos que o anfitrião do DumbMonit confie na sua autoridade certificadora.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Nome de utilizador / senha | O utilizador dedicado com a função Read-only: `dumbmonit@vsphere.local` num vCenter, `dumbmonit` num anfitrião. |

Endereço: um nome de anfitrião ou IP (`vcenter.lan`), `anfitrião:porta`, ou
um URL completo; um `/sdk` ou `/ui` final é aceite. Opções: porta (443),
verificação de certificado, tempo limite do pedido (20 s) e alarmes
disparados (lido).
