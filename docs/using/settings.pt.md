# Definições {#settings}

![Settings: conta, utilizadores, sign-on único, agentes, assistente, aparência, sobre](../assets/screenshots/settings-light.png){ loading=lazy }

Settings contém o que é administrativo: a sua conta, quem pode iniciar
sessão e como, os tokens que deixam agentes e assistentes entrarem,
fazer cópia de segurança da instância, o tema, e a saúde do servidor.
Uma página, uma secção por tema, com uma barra de âncoras num ecrã
largo e uma faixa de chips num telefone. Cada secção pode ser ligada
(`/settings#agents`).

Duas coisas viviam aqui e mudaram-se, porque não são administração:

| Procura por | Agora em | Ligação antiga |
|---|---|---|
| Canais de notificação, horas de silêncio, política de notificação | **Alerts → Notifications** ([Página de alertas](alerts.md#notifications)) | `/settings#notifications`, `/settings#notifications-policy` |
| Páginas de estado e anúncios | **Status** ([Páginas de estado](status-pages.md)) | `/settings#status` |

As ligações antigas continuam a funcionar: redirecionam para o novo
lugar.

## Account & security {#account-security}

Esta é a sua própria conta. A primeira é criada no primeiro arranque,
e mais podem ser adicionadas abaixo. Aqui altera a sua senha: senha
atual, senha nova (pelo menos 12 caracteres; uma frase inteira é mais
segura do que uma palavra complicada), confirmação. Alterá-la termina
toda outra sessão. As contas que iniciam sessão através do fornecedor
de identidade não têm senha aqui. **Sign out** termina esta sessão.

O início de sessão é limitado por taxa por endereço de cliente e por
conta: depois de cinco tentativas falhadas, cada tentativa seguinte
espera mais tempo (30 s, duplicando, até 5 minutos). Atrás de um proxy
inverso, defina `DUMBMONIT_TRUSTED_PROXIES` para que o limitador veja
o endereço real do cliente em vez do do proxy (ver
[Configuração](../reference/configuration.md)). As sessões duram 30
dias. Uma senha esquecida é reposta com
`DUMBMONIT_RESET_PASSWORD=1`: ver as
[Perguntas frequentes](../faq.md#i-lost-the-password).

### Autenticação de dois fatores {#two-factor-authentication}

As contas que iniciam sessão com senha podem adicionar um segundo
fator: um código de seis dígitos baseado no tempo (TOTP) de uma app
autenticadora — Aegis, FreeOTP, Google Authenticator, 1Password,
Bitwarden… **Set up** pede a sua senha, mostra um código QR para
digitalizar (ou a chave para escrever), e ativa o segundo fator uma
vez que introduza um primeiro código da app. Oito **códigos de
recuperação** são então mostrados uma vez: guarde-os, cada um inicia
a sua sessão uma única vez se o telefone se perder. O ecrã de início
de sessão pede o código (ou um código de recuperação) depois da
senha; cinco códigos errados cancelam a tentativa e recomeça a partir
da senha, e o limite de taxa habitual de início de sessão continua a
contar entre tentativas. Um código é aceite uma vez: o que acabou de
usar (incluindo o que ativou o segundo fator) não inicia a sua sessão
de novo — espere pelo seguinte.

**Disable** pede a senha de novo. Os códigos de recuperação não podem
ser regenerados por si só: desative e configure de novo para obter um
conjunto fresco. Um administrador pode repor o segundo fator de outro
utilizador em **Users** (*Reset 2FA*) quando o telefone e os códigos
desaparecem — isso termina a sessão do utilizador em todo lado e
deixa a senha intacta. O segredo é guardado encriptado com o segredo
da instância; os códigos têm hash. As contas de sign-on único recebem
o seu segundo fator do fornecedor de identidade, não aqui.

Os administradores também encontram o **registo de segurança** nesta
secção: inícios de sessão e falhas, alterações de senha e dois
fatores, alterações de token e conta, com o endereço do cliente.
Mantém as últimas 5000 entradas.

## Push notifications {#push-notifications}

Notificações nativas neste telefone ou computador, mesmo com o
DumbMonit fechado: **Enable on this device**, depois **Send a test**.
Cada conta gere os seus próprios dispositivos aqui (leitores também);
quais alertas os alcançam é definido por um canal **Web Push** em
Alerts → Notifications. O botão **i** resume os requisitos: HTTPS (ou
localhost), instalação no ecrã principal no iPhone e iPad, e os
domínios do serviço push que o servidor tem de alcançar. Guia
completo: [Notificações push (PWA)](push-notifications.md).

## Users {#users}

Só administradores. As contas que podem iniciar sessão, e a sua
função:

| Função | Lê tudo | Trata alertas (confirmar, adiar, ignorar, janelas de manutenção, limpar histórico) | Altera a configuração (dispositivos, regras, canais, páginas de estado, definições, cópias de segurança, tokens, contas) |
|---|---|---|---|
| **Viewer** | sim | não | não |
| **Operator** | sim | sim | não |
| **Admin** | sim | sim | sim |

Um leitor vê toda página com todo controlo que alteraria algo
escondido. Um operador é a função de prevenção: os controlos de
alerta estão lá, os ecrãs de configuração lêem-se *Operator — config
is admin only*. Só um administrador cria contas ou altera uma função
— um operador não pode promover ninguém, incluindo a si próprio. As
secções *Users*, *Single sign-on* e o registo de segurança
permanecem só de administrador. **Add user** pede um nome de
utilizador, um nome de exibição opcional, uma função e uma senha; o
botão junto ao campo de senha inventa uma aleatória. A senha é
mostrada uma vez, para entregar.

Cada linha transporta a função, se a conta inicia sessão com senha ou
através do fornecedor de identidade, se os dois fatores estão
ativados, e o último início de sessão. **Edit** altera o nome de
exibição, a função e a senha (uma reposição termina a sessão desse
utilizador em todo lado e mostra a nova senha uma vez). O interruptor
**Enabled** suspende uma conta sem a eliminar; **Reset 2FA** aparece
quando a conta tem um segundo fator; **Delete** remove-a de vez. O
servidor mantém pelo menos um administrador ativo, e os controlos
refletem essa regra em vez de a reportar como um erro: o último
administrador não pode ser despromovido, desativado ou eliminado, e
não pode desativar ou eliminar a sua própria conta.

## Single sign-on {#single-sign-on}

Só administradores. Início de sessão através de um fornecedor OpenID
Connect (Authelia, Authentik, Keycloak, Pocket ID, Google
Workspace…). Preencha o *nome do fornecedor* (etiqueta o botão
«Continue with …» no ecrã de início de sessão), o *URL do emissor* —
a descoberta é lida de `{issuer}/.well-known/openid-configuration` —
o *id de cliente*, o *segredo de cliente* (guardado encriptado, nunca
mostrado de novo) e o *URL público* onde os navegadores alcançam o
DumbMonit. O formulário constrói o URI de redirecionamento a partir
desse URL público: copie-o e registe-o com o seu fornecedor.

**Test discovery** lê a configuração do fornecedor sem iniciar
sessão em ninguém, lista os endpoints que encontrou, e avisa quando o
fornecedor assina nem com RS256 nem com ES256 — as únicas duas
assinaturas que o DumbMonit aceita. As definições guardadas aqui têm
precedência sobre as variáveis de ambiente `DUMBMONIT_OIDC_*`, que só
se aplicam enquanto nada está guardado; **Forget saved settings**
limpa-as e volta às variáveis quando existem.

**Advanced.** O resto do formulário fica sob *Advanced*: *Scopes*
nomeia o que pedir ao fornecedor, *Groups claim* a reivindicação do
token que lista os grupos do utilizador, e *Create accounts on first
sign-in* decide se uma pessoa desconhecida recebe uma conta. Está
**desligado por predefinição**: só contas que já existem podem
iniciar sessão até o ativar.

**Roles.** *Admin groups* lista os grupos cujos membros se tornam
administradores; *Operator groups* os grupos cujos outros membros se
tornam operadores; todos os outros são leitores. Deixe ambos vazios
para continuar a gerir funções em **Users**. Só com *Operator groups*
preenchido, os grupos nunca tocam na função de administrador: os
administradores continuam administradores, os outros tornam-se
operadores ou leitores. As funções são reavaliadas em cada início de
sessão, exceto que o último administrador ativo nunca é despromovido.
As variáveis de ambiente correspondentes são
`DUMBMONIT_OIDC_ADMIN_GROUPS` e `DUMBMONIT_OIDC_OPERATOR_GROUPS`.

**Em que conta aterra um início de sessão.** A identidade é lembrada
pelo seu assunto de fornecedor depois do primeiro início de sessão.
Num primeiro início de sessão:

- Uma conta existente só é ligada quando o fornecedor afirma
  `email_verified: true` e esse email é o nome de utilizador da
  conta. Um `preferred_username` ou um email não verificado nunca
  liga: na maioria dos fornecedores o utilizador escolhe esses, por
  isso `admin` do fornecedor não deve tornar-se *o seu* `admin`.
- Um **administrador local que tem uma senha nunca é ligado
  automaticamente**, mesmo num email verificado: o início de sessão
  cria uma conta distinta em vez disso. Continue a gerir esse
  administrador com a sua senha, ou crie um administrador SSO
  separado através dos grupos de administrador.
- Caso contrário, com *Create accounts on first sign-in* ativado, uma
  nova conta é criada (nome de utilizador do fornecedor, sufixado
  `-1`, `-2`… quando já ocupado). Com ele desligado, só a ligação de
  email verificado acima pode iniciar sessão.

**A criação de contas está desligada a menos que a escolha.** As
instalações novas começam com *Create accounts on first sign-in*
desligado, e também qualquer configuração onde nunca foi definido —
definições guardadas antes de a opção existir, ou configurações só
de ambiente sem `DUMBMONIT_OIDC_AUTO_CREATE`. Versões anteriores
tratavam «nunca definido» como ligado; o painel Single sign-on agora
diz quando esta predefinição se aplica. Um valor que guardou
explicitamente é mantido como está.

## Agents {#agents}

Tokens de registo para o
[agente Linux, macOS, FreeBSD e Windows](../devices/agent.md). Crie
um com um nome («File server», «Home fleet»): o token é mostrado uma
vez, com os comandos de instalação Linux e Windows prontos para
copiar. Cada comando transporta os checksums SHA-256 dos binários de
agente que este servidor distribui, e o instalador recusa uma
descarga que não corresponda; os checksums também são listados sob
os comandos, para comparar manualmente. A lista mostra o prefixo de
cada token, data de criação, última utilização e se foi revogado.
**Revoke** para todo agente a usar esse token na sua próxima
publicação.

Um token pode registar várias máquinas. Revogá-lo não elimina os
dispositivos. Os leitores veem a lista e a secção lê-se *Viewer —
read only*; criar e revogar são para administradores.

## API & assistants {#api-assistants}

Um tipo de token para ambos: clientes MCP e a
[API REST](../reference/api.md). Nomeie um token, escolha o seu
âmbito e crie-o — é mostrado uma vez. Um token **read** só pode
olhar; um token **read and write** também pode alterar coisas —
silenciar um dispositivo, executar uma verificação, adicionar ou
editar dispositivos, regras e canais — mas nunca contas, definições
de início de sessão ou outros tokens.

Em **Connect**, três separadores contêm os trechos, preenchidos com o
token enquanto ainda está no ecrã: *Claude* (o comando `claude mcp
add` e o bloco `claude_desktop_config.json`), *ChatGPT* (o URL do
servidor MCP e o cabeçalho de autorização, com um aviso quando esta
página não é servida via HTTPS — o ChatGPT liga-se a partir dos
servidores da OpenAI, por isso o endereço tem de ser acessível a
partir da internet) e *Cursor / other*, a mesma forma para qualquer
cliente que fala Streamable HTTP com um cabeçalho bearer. **API
access** mostra o mesmo token na API REST como um exemplo `curl`, e
aponta para as rotas que um Prometheus ou um Grafana consultam com
ele. Ver [Ligar um assistente](assistant.md) e
[Consultar o DumbMonit](../reference/metrics.md#scraping-dumbmonit).

## Backup {#backup}

Três coisas, pela ordem em que as encontra.

Um **aviso que não pode perder**: `/data/secret.key` é o que
desencripta as credenciais de dispositivo, e uma cópia da base de
dados sem ele restaura uma instância que não consegue falar com
nada. A formulação muda quando o segredo vem de `DUMBMONIT_SECRET`
em vez disso — não há então nenhum ficheiro para copiar, e é o seu
gestor de senhas que tem de o guardar.

**Export the configuration** lista o que o pacote conteria, com uma
contagem por secção, depois pede uma frase-passe duas vezes. O
ficheiro que desce contém toda credencial desta instância, encriptado
só com essa frase-passe: guarde-o onde guarda senhas. As senhas de
conta e os segredos de dois fatores são deixados de fora a menos que
ative o interruptor.

**Restore a bundle** toma o ficheiro e a sua frase-passe e mostra
primeiro um **ensaio**: uma tabela do que seria criado, atualizado e
deixado em paz, secção por secção, com uma nota para qualquer coisa
que não possa colocar. Nada é escrito até pressionar *Restore for
real*.

**Scheduled local backups** mostra se estão ativas, para onde vão, a
última execução e se teve sucesso, e os ficheiros mantidos com o seu
tamanho. **Back up now** escreve uma de imediato — a coisa a fazer
antes de uma atualização.

Tudo aqui é só de administrador. Ver
[Cópia de segurança e restauro](../install/backup.md).

## Reports {#reports}

Emails periódicos de disponibilidade para a equipa: escolha semanal,
diário ou mensal, o dia, hora e fuso horário, os destinatários e o
canal de email, depois **Send a preview** para ver o resultado. Só de
administrador. Ver [Relatórios por email](reports.md).

## Appearance {#appearance}

Três telhas, cada uma pré-visualizando o seu próprio tema:
**System** (segue o seu dispositivo e muda com ele), **Day** (o tema
«papel de gráfico») e **Night** (o tema «radar»). A escolha é
guardada no navegador. O alternador do cabeçalho e o *Toggle theme*
da paleta de comandos mudam entre dia e noite.

**Open wall mode** fica no fundo da secção: só o boletim, em tela
cheia, para um monitor na sala. ++esc++ sai dele, e a paleta de
comandos abre-o de qualquer lugar — ver [Modo parede](overview.md#wall-mode).

## About {#about}

Versão do servidor e a saúde dos seus dois armazenamentos, a base de
dados (configuração de dispositivo e histórico de alertas) e a
VictoriaMetrics (toda medição nos gráficos), como reportado por
`GET /api/health` e atualizado a cada 15 segundos. Um armazenamento
que deixa de responder mostra um aviso com o erro e o que verificar.
A secção também liga a esta documentação e nomeia a licença, Apache
2.0.
