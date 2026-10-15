# FortiGate {#fortigate}

Um Fortinet FortiGate através da API REST do FortiOS: túneis IPsec
site-to-site em baixo, interfaces ativadas que perderam a sua ligação, um
cluster HA fora de sincronia ou com um membro em falta, memória a
aproximar-se do modo de conservação, licenças FortiGuard e FortiCare a
expirar, e versões de correção oferecidas para o ramo FortiOS em execução.

O DumbMonit usa um utilizador REST API cujo perfil de acesso só lê, e que só
aceita chamadas do endereço do servidor DumbMonit (anfitrião de confiança).
O seu token é enviado como `Authorization: Bearer`, nunca no URL: o FortiOS
7.4.5 e 7.6.1 recusam-no aí de qualquer forma.

A cada minuto o DumbMonit lê três páginas obrigatórias —
`/api/v2/monitor/system/status`, `/api/v2/monitor/system/resource/usage` e
`/api/v2/monitor/system/interface/select` — e seis opcionais que um perfil
mais restrito, ou um FortiGate sem cluster ou túneis, pode recusar ou deixar
vazias sem fazer o resto falhar: o estado administrativo das interfaces
(`/api/v2/cmdb/system/interface`), estatísticas e checksums de HA, túneis
IPsec, estado da licença e firmware disponível.

!!! warning "Validado apenas contra a documentação do fornecedor — ainda não testado num sistema real"

    Esta integração foi construída a partir da documentação FortiOS da
    Fortinet e das formas de resposta registadas pelo exportador Prometheus
    da comunidade para FortiGate, não contra um FortiGate em funcionamento.
    Toda chamada é uma leitura. As páginas de licença e firmware são as
    menos documentadas: diga-nos o que falha.

## O que monitoriza {#what-it-watches}

Todas as métricas têm o prefixo `dumbmonit_fortigate_`.

| Métrica | Etiquetas | Significado |
|---|---|---|
| `version_info` | `version`, `build`, `model`, `hostname` | Valor 1. |
| `cpu_usage_percent`, `memory_used_percent`, `disk_used_percent` | | Uso atual. |
| `sessions`, `session_setup_rate` | | Sessões ativas, novas sessões por segundo. |
| `log_disk_ok` | | 0 quando o disco de logs precisa de formatação. |
| `interface_link_up` | `interface`, `alias` | Ligação física. |
| `interface_enabled` | `interface`, `alias` | Estado administrativo; toda interface conta como ativada quando o perfil não pode ler a configuração. |
| `interface_down` | `interface`, `alias` | 1 quando a interface está ativada, tem um endereço e não tem ligação. Portas livres sem endereço são deixadas em paz. |
| `interface_speed_mbps` | `interface`, `alias` | Enquanto a ligação está ativa. |
| `interface_bytes_in_total`, `interface_bytes_out_total`, `interface_errors_in_total`, `interface_errors_out_total` | `interface`, `alias` | Contadores. |
| `interfaces_down` | | Contagem. |
| `ha_members` | | Membros no cluster; 0 para um FortiGate autónomo. |
| `ha_in_sync` | | Com dois membros ou mais: 1 quando todo membro tem o mesmo checksum de configuração. |
| `ha_member_cpu_usage_percent`, `ha_member_memory_used_percent` | `member` | Por membro. |
| `ipsec_tunnel_up` | `tunnel` | 1 quando pelo menos uma fase 2 do túnel está ativa. Túneis dial-up (acesso remoto) são deixados de fora. |
| `ipsec_phase2_up` | `tunnel`, `phase2` | Por seletor de fase 2. |
| `ipsec_tunnel_bytes_in_total`, `ipsec_tunnel_bytes_out_total` | `tunnel` | Contadores. |
| `ipsec_tunnel_info` | `tunnel`, `remote_gateway` | Valor 1. |
| `ipsec_tunnels`, `ipsec_tunnels_down` | | Contagens. |
| `license_status_info` | `license`, `status` | Valor 1, para toda licença que o FortiGate detém (`antivirus`, `ips`, `web_filtering`, `forticare_hardware`…); entradas `no_license` são deixadas de fora. |
| `license_expired` | `license` | 1 quando o estado é `expired`. |
| `license_expiry_seconds` | `license` | Até à expiração; negativo depois de expirado. |
| `firmware_update_available` | | 1 quando o FortiGuard oferece uma versão de correção mais recente do ramo em execução (7.4.4 → 7.4.5). Um ramo novo (7.6) é um projeto, não um alerta. |
| `firmware_latest_info` | `version` | Valor 1: essa versão de correção. |

As [regras incorporadas](../alerting/rules.md#fortigate) que se aplicam:

- **FortiGate IPsec tunnel down**: nenhuma fase 2 ativa há cinco minutos (Warning).
- **FortiGate interface without link**: durante cinco minutos (Advisory).
- **FortiGate HA out of sync**: durante quinze minutos (Advisory).
- **FortiGate HA member lost**: menos membros do que em qualquer momento
  nas últimas 24 horas (Warning). A contagem esperada não é publicada em
  lado nenhum, por isso um membro removido definitivamente deixa de
  alertar no dia seguinte.
- **FortiGate memory near conserve mode**: acima de 85% durante dez
  minutos (Advisory). A 88% por predefinição o FortiOS entra em modo de
  conservação e para de inspecionar novas sessões.
- **FortiGate CPU high**: acima de 90% durante quinze minutos (Advisory).
- **FortiGate licence expiring**: menos de 30 dias restantes, ou expirada
  (Advisory, lembrado diariamente).
- **FortiGate firmware update**: uma versão de correção é oferecida (Info,
  lembrado semanalmente).

## A página do monitor {#the-device-page}

O painel Firewall diz o que está mal numa frase, mostra CPU, memória,
sessões e o estado HA, depois lista os túneis IPsec (os em baixo primeiro),
as interfaces sem ligação, as licenças a expirar dentro de 60 dias ou
expiradas, e uma versão de correção disponível.

## Criar um utilizador REST API só de leitura para o DumbMonit {#create-a-read-only-rest-api-user-for-dumbmonit}

1. A partir da CLI do FortiGate, crie um perfil de acesso que só pode ler o estado do sistema, rede e VPN, e nada mais.

    ```
    config system accprofile
    edit "dumbmonit-ro"
    set sysgrp read
    set netgrp read
    set vpngrp read
    set fwgrp none
    set loggrp none
    set utmgrp none
    set wanoptgrp none
    set wifi none
    next
    end
    ```

2. Crie o utilizador REST API dumbmonit com esse perfil, confiando só no endereço do servidor DumbMonit (substitua 192.0.2.10), depois gere o seu token: é impresso uma vez.

    ```
    config system api-user
    edit "dumbmonit"
    set accprofile "dumbmonit-ro"
    set vdom "root"
    config trusthost
    edit 1
    set ipv4-trusthost 192.0.2.10 255.255.255.255
    next
    end
    next
    end
    execute api-user generate-key dumbmonit
    ```

3. No DumbMonit, introduza o endereço do FortiGate, por exemplo «fw.lan» (adicione a porta se a interface de administração não escutar na 443), e cole o token como API token. O FortiGate vem com um certificado autoassinado: marque Accept an unverifiable certificate a menos que tenha instalado o seu próprio.

!!! warning
    Uma interface conta como em baixo só quando está ativada, tem um endereço e não tem ligação: portas livres são deixadas em paz. As atualizações de firmware são as que o FortiGuard oferece ao próprio FortiGate: sem acesso ao FortiGuard, nenhuma é reportada.

## Credenciais {#credentials}

| Credencial | Campos |
|---|---|
| Token de API | O token impresso por `execute api-user generate-key dumbmonit`, enviado como `Authorization: Bearer`. |

Endereço: o nome de anfitrião ou IP do FortiGate, `anfitrião:porta`, ou um
URL. Opções:

- **Port** (443): a porta HTTPS da interface de administração (`set
  admin-sport` em `config system global`).
- **VDOM**: o domínio virtual a ler; vazio para aquele a que o utilizador
  REST API pertence.
- **Accept an unverifiable certificate**: o próprio certificado do
  FortiGate é autoassinado.
- **Timeout per request** (15 s).

## Resolução de problemas {#troubleshooting}

«FortiGate refused the API token (401)»: o token está errado ou foi
regenerado. Gere um novo com `execute api-user generate-key dumbmonit`.

«FortiGate refused … (403)»: o endereço do DumbMonit não está entre os
anfitriões de confiança do utilizador, ou o seu perfil não pode ler essa
página. Uma página opcional recusada desta forma é simplesmente deixada de
fora.

Nenhum valor de IPsec, licença ou firmware: o perfil não os pode ler, ou o
FortiGate não tem nenhum túnel; o resto continua a ser lido.
