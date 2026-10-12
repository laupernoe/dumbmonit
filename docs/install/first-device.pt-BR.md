# Adicione seu primeiro dispositivo

Adicionar qualquer coisa sempre funciona da mesma forma: escolha um tipo da
lista, um aviso à direita explica o que preparar no dispositivo, e o
formulário só mostra os campos que importam para esse tipo.

## Os três primeiros passos

Em uma instância nova, a visão geral não mostra um boletim vazio: ela
mostra um guia curto — **adicione seu primeiro dispositivo**, **conecte
uma forma de ser avisado**, **verifique que uma mensagem chega**. Cada
passo é um clique, e cada um fica verde por conta própria. O terceiro só
conta uma mensagem que realmente saiu de um canal, então criar um canal e
nunca testá-lo não o marca.

O guia é mantido na instância, não no navegador: pular um passo, ou o guia
inteiro, o pula em todo navegador e para todo usuário. Ele desaparece de
vez uma vez que os três passos estão resolvidos, e nunca aparece em uma
instância que já tinha dispositivos e um canal — atualizar não entrega um
tutorial a um homelab já estabelecido.

## Adicionar um dispositivo SNMP

O SNMP é o tipo mais universal: quase todo equipamento de rede o fala.
Switches, roteadores, NAS, nobreaks e impressoras são os suspeitos
habituais.

1. No dispositivo, ative o SNMP v2c e anote a comunidade somente leitura
   (`public` por padrão em muitos dispositivos). Se o dispositivo filtra
   por endereço, permita o endereço do servidor DumbMonit.
2. No DumbMonit, abra **Dispositivos** e clique em **Adicionar
   dispositivo** (ou pressione ++ctrl+k++ e digite "add").
3. Escolha **Dispositivo SNMP**. O seletor se recolhe em uma linha e o
   formulário mostra os campos para SNMP.
4. Informe um **nome**, o **endereço** (IP ou nome de host) e a
   **comunidade**. Para SNMP v3, mude a credencial para *SNMP v3* e
   preencha o nome de usuário e as frases secretas de autenticação e
   privacidade.
5. Clique em **Adicionar dispositivo**.

![Adicionando um dispositivo SNMP: o formulário à esquerda, o aviso de configuração à direita](../assets/screenshots/add-device-light.png){ loading=lazy }

O perfil de coleta é detectado automaticamente a partir do `sysObjectID` do
dispositivo, em segundo plano, então o formulário nunca espera por um
dispositivo inacessível. Interfaces, e em servidores e NAS a CPU, memória e
armazenamento, aparecem dentro de um minuto.

!!! warning "Comunidades viajam em texto claro"
    Uma comunidade SNMP v1/v2c não é criptografada na rede. Em uma rede
    compartilhada, prefira SNMP v3, que autentica e criptografa.

## Varrer uma rede

Em vez de digitar dispositivos um a um, varra uma faixa:

1. Em **Adicionar dispositivo**, abra o painel **Varrer minha rede**.
2. Informe uma rede em notação CIDR, por exemplo `192.168.1.0/24` (até
   4096 endereços por varredura), e a comunidade a tentar em cada endereço
   (`public` por padrão).
3. Todo endereço que responde é listado com seu `sysName`, descrição e o
   perfil que seria aplicado. Dispositivos já no DumbMonit são marcados
   *Já adicionado*, e tudo novo é marcado para você.
4. Desmarque o que você não quer, depois clique em **Adicionar *n*
   dispositivos**. Eles são criados um a um, cada linha mostrando seu
   próprio resultado, então uma falha nunca para o resto.

## Adicionar um servidor Proxmox VE

O mesmo fluxo, com uma credencial em dois campos em vez de um:

1. No Proxmox, crie um usuário reservado para monitoramento, um papel
   somente leitura e um token de API — os quatro comandos `pveum` estão no
   aviso ao lado do formulário, com um botão de copiar cada, e na
   [página do Proxmox VE](../devices/proxmox.md). Nunca reutilize a conta
   com a qual você faz login.
2. No DumbMonit, **Adicionar dispositivo** → **Proxmox VE**.
3. Informe um **nome** e o **endereço** de qualquer nó (porta 8006 por
   padrão).
4. Copie o **full-tokenid** do token (`dumbmonit@pve!monitor`) para **Token
   ID** e seu **valor** (o UUID) para **Secret**. Colou a string completa
   `user@pve!name=secret` em Token ID por engano? O formulário a separa e
   avisa isso.
5. O Proxmox vem com um certificado autoassinado: marque **Aceitar um
   certificado não verificável** sob *Mais opções* se a primeira sondagem
   reclamar dele.
6. Clique em **Adicionar dispositivo**.

O [Proxmox Backup Server](../devices/pbs.md) funciona da mesma forma com
`proxmox-backup-manager`; um [NAS Synology](../devices/synology.md) toma
uma conta DSM dedicada (nome de usuário e senha); um
[servidor com o agente](../devices/agent.md) não precisa de credencial
alguma, apenas o token de cadastro mostrado uma vez que o dispositivo é
salvo.

## O aviso de configuração

Todo tipo carrega seu próprio aviso, escrito pelo servidor e mostrado ao
lado do formulário: passos numerados a fazer no dispositivo (criar um
usuário somente leitura, criar um token de API, permitir o endereço…), com
o caminho de menu exato no produto e um botão de copiar em todo comando ou
valor, depois uma armadilha comum e um link para a documentação do
fornecedor quando existe uma. As páginas de cada tipo nesta documentação
carregam os mesmos passos, palavra por palavra — um teste no servidor os
mantém idênticos: [SNMP](../devices/snmp.md),
[Proxmox VE](../devices/proxmox.md),
[Proxmox Backup Server](../devices/pbs.md),
[Synology DSM](../devices/synology.md), [agente](../devices/agent.md),
[serviços](../devices/services.md).

O princípio detrás de todo aviso: uma conta dedicada somente leitura, nunca
a que você usa para entrar. Um segredo de monitoramento exposto não deve
conseguir alterar nada.

## Campos comuns

Qualquer que seja o tipo, o formulário tem:

| Campo | Significado |
|---|---|
| Nome | Como o dispositivo aparece em listas e alertas. |
| Endereço | IP, nome de host, `host:porta` ou URL dependendo do tipo; o placeholder mostra a forma esperada. |
| Credencial | As famílias que o tipo aceita, cada uma com seus próprios campos: comunidade, usuário SNMP v3 e frases secretas, token Proxmox (Token ID + Secret), nome de usuário e senha. Segredos têm um olho de mostrar/ocultar e valores colados são limpos de espaços, aspas e quebras de linha perdidos. Armazenado criptografado, nunca retornado pela API: ao editar, campos vazios mantêm o que está salvo. |
| Intervalo de verificação | Com que frequência o DumbMonit lê o dispositivo. Padrão 60 s, mínimo 10 s. |
| Dispositivo pai | Se o pai cair, alertas deste dispositivo são suprimidos em vez de enviados. Veja [supressão por dependência](../alerting/index.md#dependency-suppression). |
| Ativado | Um dispositivo pausado não é verificado e não gera alertas. |
| Tags | Rótulos livres chave/valor, copiados em toda série como `tag_<key>`. Opções específicas do tipo também vivem aqui, sob **Mais opções**. |

## O que acontece depois

- O dispositivo aparece na página **Dispositivos** como uma placa de rack:
  LED de status, nome, tipo, endereço, visto por último, e um sparkline.
- Medições são gravadas no VictoriaMetrics a cada 5 segundos
  (`DUMBMONIT_FLUSH_INTERVAL_SECS`), então o primeiro gráfico aparece
  depois da primeira sondagem mais alguns segundos. **Sondar agora** na
  página do dispositivo força uma sondagem e reporta quantas amostras e
  séries ela produziu: é a principal ferramenta de diagnóstico.
- As [regras de alerta integradas](../alerting/rules.md) se aplicam
  imediatamente: dispositivo inacessível, CPU saturada, disco quase cheio,
  e assim por diante. Nada para configurar.
- Para ser notificado, adicione um
  [canal de notificação](../notifications.md) em **Alertas →
  Notificações**. Regras integradas notificam todo canal ativado.

!!! tip "Sem dados depois de um minuto?"
    Abra a página do dispositivo: um erro de configuração (comunidade
    errada, certificado recusado, capacidade faltando) é mostrado em
    vermelho abaixo do endereço, com o motivo. Veja as
    [Perguntas frequentes](../faq.md#no-data-after-adding-a-device).
