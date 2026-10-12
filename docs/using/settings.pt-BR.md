# Configurações

![Configurações: conta, usuários, login único, agentes, assistente, aparência, sobre](../assets/screenshots/settings-light.png){ loading=lazy }

Configurações mantém o que é administrativo: sua conta, quem pode entrar e
como, os tokens que permitem agentes e assistentes entrarem, fazer backup
da instância, o tema, e a saúde do servidor. Uma página, uma seção por
tópico, com uma trilha de âncoras em uma tela larga e uma faixa de chips em
um celular. Cada seção pode ser compartilhada por link (`/settings#agents`).

Duas coisas costumavam viver aqui e se mudaram, porque não são
administração:

| Procurando por | Agora sob | Link antigo |
|---|---|---|
| Canais de notificação, horas de silêncio, política de notificação | **Alertas → Notificações** ([Página de alertas](alerts.md#notifications)) | `/settings#notifications`, `/settings#notifications-policy` |
| Páginas de status e anúncios | **Status** ([Páginas de status](status-pages.md)) | `/settings#status` |

Os links antigos ainda funcionam: eles redirecionam para o novo lugar.

## Conta e segurança

Esta é a sua própria conta. A primeira é criada na primeira inicialização,
e mais podem ser adicionadas abaixo. Aqui você muda sua senha: senha
atual, nova senha (pelo menos 12 caracteres; uma frase inteira é mais
segura que uma palavra complicada), confirmação. Mudá-la desconecta toda
outra sessão. Contas que entram através do provedor de identidade não têm
senha aqui. **Sair** termina esta sessão.

O login tem limite de taxa por endereço de cliente e por conta: após cinco
tentativas falhadas, cada tentativa adicional espera mais (30 s, dobrando,
até 5 minutos). Detrás de um proxy reverso, defina
`DUMBMONIT_TRUSTED_PROXIES` para que o limitador veja o endereço real do
cliente em vez do do proxy (veja
[Configuração](../reference/configuration.md)). Sessões duram 30 dias.
Uma senha esquecida é redefinida com `DUMBMONIT_RESET_PASSWORD=1`: veja as
[Perguntas frequentes](../faq.md#i-lost-the-password).

### Autenticação de dois fatores

Contas que entram com senha podem adicionar um segundo fator: um código de
seis dígitos baseado em tempo (TOTP) de um app autenticador — Aegis,
FreeOTP, Google Authenticator, 1Password, Bitwarden… **Configurar** pede
sua senha, mostra um código QR para escanear (ou a chave para digitar), e
ativa o segundo fator uma vez que você informa um primeiro código do app.
Oito **códigos de recuperação** são então mostrados uma vez: salve-os,
cada um entra você uma única vez se o celular for perdido. A tela de login
pede o código (ou um código de recuperação) depois da senha; cinco códigos
errados cancelam a tentativa e você recomeça da senha, e o limite de taxa
de login habitual continua contando entre tentativas. Um código é aceito
uma vez: o que você acabou de usar (incluindo o que ativou o segundo
fator) não entra você de novo — espere pelo próximo.

**Desativar** pede a senha de novo. Códigos de recuperação não podem ser
regenerados por conta própria: desative e configure de novo para obter um
conjunto novo. Um administrador pode redefinir o segundo fator de outro
usuário em **Usuários** (*Redefinir 2FA*) quando tanto o celular quanto os
códigos se perderam — isso desconecta o usuário em todo lugar e deixa a
senha intacta. O segredo é armazenado criptografado com o segredo da
instância; códigos são hasheados. Contas de login único recebem seu
segundo fator do provedor de identidade, não aqui.

Administradores também encontram o **log de segurança** nesta seção:
logins e falhas, mudanças de senha e dois fatores, mudanças de token e
conta, com o endereço do cliente. Ele mantém as últimas 5.000 entradas.

## Notificações push

Notificações nativas neste celular ou computador, mesmo com o DumbMonit
fechado: **Ativar neste dispositivo**, depois **Enviar um teste**. Cada
conta gerencia seus próprios dispositivos aqui (visualizadores também);
quais alertas chegam até eles é definido por um canal **Web Push** em
Alertas → Notificações. O botão **i** resume os requisitos: HTTPS (ou
localhost), instalação na Tela de Início no iPhone e iPad, e os domínios
de serviço de push que o servidor precisa alcançar. Guia completo:
[Notificações push (PWA)](push-notifications.md).

## Usuários

Apenas administradores. As contas que podem entrar, e seu papel:

| Papel | Lê tudo | Lida com alertas (reconhecer, soneca, ignorar, janelas de manutenção, limpar histórico) | Altera a configuração (dispositivos, regras, canais, páginas de status, configurações, backups, tokens, contas) |
|---|---|---|---|
| **Visualizador** | sim | não | não |
| **Operador** | sim | sim | não |
| **Admin** | sim | sim | sim |

Um visualizador vê toda página com todo controle que mudaria algo
oculto. Um operador é o papel de plantão: os controles de alerta estão
lá, as telas de configuração leem *Operador — configuração é restrita a
administradores*. Só um admin cria contas ou muda um papel — um operador
não pode promover ninguém, incluindo a si mesmo. As seções *Usuários*,
*Login único* e o log de segurança permanecem restritas a administradores.
**Adicionar usuário** pede um nome de usuário, um nome de exibição
opcional, um papel e uma senha; o botão ao lado do campo de senha inventa
uma aleatória. A senha é mostrada uma vez, para ser entregue.

Cada linha carrega o papel, se a conta entra com senha ou através do
provedor de identidade, se o segundo fator está ativo, e o último login.
**Editar** muda o nome de exibição, o papel e a senha (uma redefinição
desconecta esse usuário em todo lugar e mostra a nova senha uma vez). O
botão **Ativado** suspende uma conta sem excluí-la; **Redefinir 2FA**
aparece quando a conta tem um segundo fator; **Excluir** a remove de vez.
O servidor mantém pelo menos um administrador ativo, e os controles
refletem essa regra em vez de reportá-la como um erro: o último
administrador não pode ser rebaixado, desativado ou excluído, e você não
pode desativar ou excluir sua própria conta.

## Login único

Apenas administradores. Login através de um provedor OpenID Connect
(Authelia, Authentik, Keycloak, Pocket ID, Google Workspace…). Preencha o
*nome do provedor* (rotula o botão "Continuar com …" na tela de login), a
*URL do emissor* — a descoberta é lida de
`{issuer}/.well-known/openid-configuration` — o *id do cliente*, o
*segredo do cliente* (armazenado criptografado, nunca mostrado de novo) e
a *URL pública* onde navegadores alcançam o DumbMonit. O formulário
constrói a URI de redirecionamento a partir dessa URL pública: copie-a e a
registre com seu provedor.

**Testar descoberta** lê a configuração do provedor sem conectar ninguém,
lista os endpoints que encontrou, e avisa quando o provedor assina com
nem RS256 nem ES256 — as únicas duas assinaturas que o DumbMonit aceita.
Configurações salvas aqui têm precedência sobre as variáveis de ambiente
`DUMBMONIT_OIDC_*`, que só se aplicam enquanto nada está salvo;
**Esquecer configurações salvas** as limpa e volta às variáveis quando
existem.

**Avançado.** O resto do formulário fica sob *Avançado*: *Escopos* nomeia
o que pedir ao provedor, *Reivindicação de grupos* a reivindicação do
token que lista os grupos do usuário, e *Criar contas no primeiro login*
decide se uma pessoa desconhecida recebe uma conta. Está **desligado por
padrão**: só contas que já existem podem entrar até você ativá-lo.

**Papéis.** *Grupos de admin* lista os grupos cujos membros se tornam
administradores; *Grupos de operador* os grupos cujos outros membros se
tornam operadores; todos os demais são visualizadores. Deixe ambos vazios
para continuar gerenciando papéis sob **Usuários**. Com apenas *Grupos de
operador* preenchido, os grupos nunca tocam o papel de admin: admins
permanecem admins, os outros se tornam operadores ou visualizadores.
Papéis são reavaliados a cada login, exceto que o último admin ativo
nunca é rebaixado. As variáveis de ambiente correspondentes são
`DUMBMONIT_OIDC_ADMIN_GROUPS` e `DUMBMONIT_OIDC_OPERATOR_GROUPS`.

**Em qual conta um login cai.** A identidade é lembrada por seu assunto de
provedor depois do primeiro login. Em um primeiro login:

- Uma conta existente só é vinculada quando o provedor afirma
  `email_verified: true` e esse e-mail é o nome de usuário da conta. Um
  `preferred_username` ou um e-mail não verificado nunca vincula: na
  maioria dos provedores o usuário escolhe esses, então `admin` do
  provedor não deve se tornar *seu* `admin`.
- Um **admin local que tem senha nunca é vinculado automaticamente**,
  mesmo em um e-mail verificado: o login cria uma conta distinta em vez
  disso. Continue gerenciando esse admin com sua senha, ou crie um admin
  SSO separado através dos grupos de admin.
- Caso contrário, com *Criar contas no primeiro login* ativado, uma nova
  conta é criada (nome de usuário do provedor, sufixado `-1`, `-2`…
  quando já em uso). Com desligado, só o vínculo de e-mail verificado
  acima pode entrar.

**A criação de conta está desligada a menos que você a escolha.** Novas
instalações começam com *Criar contas no primeiro login* desligado, e o
mesmo vale para qualquer configuração onde nunca foi definido —
configurações salvas antes da opção existir, ou configurações apenas de
ambiente sem `DUMBMONIT_OIDC_AUTO_CREATE`. Versões anteriores tratavam
"nunca definido" como ligado; o painel de Login único agora diz quando
esse padrão se aplica. Um valor que você salvou explicitamente é mantido
como está.

## Agentes

Tokens de cadastro para o
[agente Linux, macOS, FreeBSD e Windows](../devices/agent.md). Crie um com
um nome ("Servidor de arquivos", "Frota doméstica"): o token é mostrado uma
vez, com os comandos de instalação Linux e Windows prontos para copiar.
Cada comando carrega os checksums SHA-256 dos binários de agente que este
servidor entrega, e o instalador recusa um download que não corresponda;
os checksums também são listados abaixo dos comandos, para comparar
manualmente. A lista mostra o prefixo de cada token, data de criação,
último uso e se foi revogado. **Revogar** para todo agente usando esse
token em seu próximo envio.

Um token pode cadastrar várias máquinas. Revogá-lo não exclui os
dispositivos. Visualizadores veem a lista e a seção lê *Visualizador —
somente leitura*; criar e revogar são para administradores.

## API e assistentes

Um tipo de token para ambos: clientes MCP e a [API REST](../reference/api.md).
Nomeie um token, escolha seu escopo e crie-o — é mostrado uma vez. Um
token **read** só pode olhar; um token **read and write** também pode
alterar coisas — silenciar um dispositivo, executar uma sondagem,
adicionar ou editar dispositivos, regras e canais — mas nunca contas,
configurações de login ou outros tokens.

Sob **Conectar**, três abas mantêm os trechos, preenchidos com o token
enquanto ainda está na tela: *Claude* (o comando `claude mcp add` e o
bloco `claude_desktop_config.json`), *ChatGPT* (a URL do servidor MCP e o
cabeçalho de autorização, com um aviso quando esta página não é servida
via HTTPS — o ChatGPT se conecta dos servidores da OpenAI, então o
endereço precisa ser acessível pela internet) e *Cursor / outro*, a mesma
forma para qualquer cliente que fala Streamable HTTP com um cabeçalho
bearer. **Acesso à API** mostra o mesmo token na API REST como um exemplo
`curl`, e aponta para as rotas que um Prometheus ou um Grafana coletam com
ele. Veja [Conectar um assistente](assistant.md) e
[Coletando do DumbMonit](../reference/metrics.md#scraping-dumbmonit).

## Backup

Três coisas, na ordem em que você as encontra.

Um **aviso que você não pode perder**: `/data/secret.key` é o que
descriptografa credenciais de dispositivo, e uma cópia do banco de dados
sem ele restaura uma instância que não pode falar com nada. O texto muda
quando o segredo vem de `DUMBMONIT_SECRET` em vez disso — não há então
nenhum arquivo para copiar, e é seu gerenciador de senhas que precisa
mantê-lo.

**Exportar a configuração** lista o que o pacote conteria, com uma
contagem por seção, depois pede uma frase secreta duas vezes. O arquivo
que vem contém toda credencial desta instância, criptografada apenas com
essa frase secreta: mantenha-o onde você mantém senhas. Senhas de conta e
segredos de 2FA são deixados de fora a menos que você ative a opção.

**Restaurar um pacote** recebe o arquivo e sua frase secreta e mostra uma
**simulação** primeiro: uma tabela do que seria criado, atualizado e
deixado intacto, seção por seção, com uma nota para qualquer coisa que não
puder colocar. Nada é escrito até você clicar em *Restaurar de verdade*.

**Backups locais programados** mostra se estão ativos, para onde vão, a
última execução e se teve sucesso, e os arquivos mantidos com seu tamanho.
**Fazer backup agora** escreve um imediatamente — a coisa a fazer antes de
uma atualização.

Tudo aqui é restrito a administradores. Veja
[Backup e restauração](../install/backup.md).

## Relatórios

E-mails periódicos de disponibilidade para a equipe: escolha semanal,
diário ou mensal, o dia, hora e fuso horário, os destinatários e o canal
de e-mail, depois **Enviar pré-visualização** para ver o resultado.
Restrito a administradores. Veja [Relatórios por e-mail](reports.md).

## Aparência

Três blocos, cada um pré-visualizando seu próprio tema: **Sistema** (segue
seu dispositivo e muda com ele), **Dia** (o tema "papel de gráfico") e
**Noite** (o tema "radar"). A escolha é armazenada no navegador. O
alternador no cabeçalho e o *Alternar tema* da paleta de comandos mudam
entre dia e noite.

**Abrir modo parede** fica na parte inferior da seção: o boletim apenas,
em tela cheia, para um monitor na sala. ++esc++ o deixa, e a paleta de
comandos o abre de qualquer lugar — veja [Modo parede](overview.md#wall-mode).

## Sobre

Versão do servidor e a saúde de seus dois armazenamentos, o banco de dados
(configuração de dispositivo e histórico de alertas) e o VictoriaMetrics
(toda medição nos gráficos), como reportado por `GET /api/health` e
atualizado a cada 15 segundos. Um armazenamento que para de responder
mostra um aviso com o erro e o que verificar. A seção também liga para
esta documentação e nomeia a licença, Apache 2.0.
