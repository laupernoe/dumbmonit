# A página de alertas {#alerts-page}

![A página de alertas: o que precisa de você, as regras, manutenção programada e histórico](../assets/screenshots/alerts-light.png){ loading=lazy }

Tudo sobre alertas em uma página, por trás de cinco abas. A aba está na URL
(`/alerts#rules`), então uma visão pode ser compartilhada por link.
**Programar manutenção**, no canto superior direito, abre o formulário para
uma janela a partir de qualquer aba.

| Aba | O que contém |
|---|---|
| **Agora** | A lista ativa *Precisa de você*, agrupada por dispositivo: placa de severidade, motivo, desde quando, e *Reconhecer*, *Soneca*, *Ignorar* e *Abrir dispositivo* em cada um. Alertas reconhecidos e em soneca ficam cada um em seu próprio grupo silencioso na parte inferior. O selo na aba, e em Alertas na barra superior, é a contagem do que ainda precisa de você. |
| **Programado** | Janelas de manutenção, *Ativa agora* ou *Programada*, pontual, semanal ou mensal, cada uma com a próxima ocorrência. Veja [Janelas de manutenção](../alerting/maintenance.md). |
| **Regras** | Toda regra com sua severidade, uma marca *Integrada* e, se algum dispositivo a ignora, *Ignorada em N dispositivos*; ativar, editar embutido, excluir as suas próprias, criar uma regra de limite. Veja [Regras](../alerting/rules.md). |
| **Notificações** | Onde os alertas chegam até você: os canais e a política de notificação. Detalhes abaixo. |
| **Histórico** | As últimas 200 transições, cada uma nomeando o dispositivo, a regra e o que aconteceu; as resolvidas podem ser limpas. |

O mesmo modelo de verdade alimenta o boletim da visão geral e esta página,
então os dois sempre concordam sobre o que precisa de você.

## Se livrando de um alerta {#getting-rid-of-an-alert}

Um alerta disparando não é binário — "resolver" ou "viver com o ruído para
sempre". Quatro ações, para quatro situações diferentes, abertas a
administradores e operadores (um visualizador vê o estado mas não pode
alterá-lo):

- **Reconhecer** ("estou cuidando disso") é para um alerta que você está
  trabalhando ativamente: "eu sei, pare de me lembrar por 4 h". O menu
  oferece 1 h, 4 h, 24 h ou *até resolver*, mais uma nota opcional para quem
  ler o cartão depois de você. O alerta continua disparando e sendo
  avaliado; apenas seus lembretes e escalonamentos pausam — incluindo o
  escalonamento para um segundo canal. Você ainda é avisado quando ele
  resolve, e o reconhecimento se limpa nesse momento — um alerta que volta
  depois notifica de novo. O cartão sai da lista principal para um grupo
  mais silencioso *Reconhecido*, lê *Reconhecido por alguém até um horário*,
  e oferece **Desfazer reconhecimento**.
- **Soneca** é para "não agora, verifique depois" sem afirmar que você está
  fazendo algo sobre isso: 1 h, 8 h, 1 dia ou *até resolver*. Diferente do
  Reconhecer, funciona abrindo uma janela de manutenção restrita aos rótulos
  exatos daquele alerta — uma regra irmã no mesmo dispositivo continua
  falando. O cartão se move para um grupo silencioso *Em soneca* mostrando
  o tempo restante, e oferece **Tirar da soneca**. Lembretes pausam da mesma
  forma que os de um reconhecimento.
- **Ignorar** ("não me alerte sobre isso de novo") é para uma regra que
  nunca fará sentido neste dispositivo: desativa essa regra apenas para esse
  dispositivo (uma substituição por dispositivo — veja
  [Substituições por dispositivo](../alerting/rules.md#per-device-overrides))
  e o alerta desaparece de vez, imediatamente, não apenas silenciado. É
  reversível: **Parar de ignorar** na página do dispositivo, ou da própria
  regra em Alertas → Regras, onde lê *Ignorada em N dispositivos*.
- **Limpar** é para o histórico, não para um alerta ativo: uma vez que uma
  transição resolveu, *Limpar* (ou *Limpar todas resolvidas*) a remove da
  visão padrão do Histórico. A linha permanece no banco de dados — nada é
  excluído — e *Mostrar limpas* a traz de volta.

Páginas de status públicas ignoram reconhecimentos e sonecas: um alerta
reconhecido ou em soneca ainda é um alerta para o mundo exterior. Uma regra
ignorada, por outro lado, nunca produziu um alerta em primeiro lugar.

## Notificações {#notifications}

Canais e a política vivem aqui — sob Alertas, não Configurações — porque
decidem quem ouve um alerta, o que é uma questão de alerta. Dois painéis:

**Canais.** A lista de canais, cada um com seu tipo, *Ativado* ou
*Desativado*, quando enviou algo por último, e seu último erro se houver.
**Adicionar canal** abre um formulário construído a partir da descrição do
servidor para o tipo escolhido: configurações (visíveis) e segredos (apenas
escrita). **Enviar teste** envia uma mensagem de teste e mostra o resultado
embutido. As **Opções de entrega** de cada canal mantêm sua severidade
mínima, se ele ouve resoluções, um intervalo mínimo por alerta, e suas
**horas de silêncio** (semanais, em seu fuso horário: apenas alertas de
nível Warning passam, o resto espera por um resumo).

**Apenas alguns alertas.** As mesmas Opções de entrega mantêm o **filtro de
roteamento** do canal: quais alertas ele quer, por tag de dispositivo
(`site=cellar`), por tipo de dispositivo (`proxmox`), ou por regra.
Condições no mesmo campo leem como *ou*, campos diferentes como *e*, e uma
linha *Exceto* sempre vence. Sob o editor, uma pré-visualização lista quais
de seus dispositivos o filtro seleciona agora, e uma frase resume tudo de
volta para você — *este canal recebe avisos e acima de dispositivos marcados
site=cellar, exceto dispositivos marcados role=lab*. Um canal sem filtro
continua recebendo tudo, o que é o que todo canal existente faz.

Editar um canal e deixar o segredo vazio mantém o armazenado. Tipos de canal
e seus campos estão documentados em
[Canais de notificação](../notifications.md).

Regras de alerta integradas notificam todo canal ativado; uma regra também
pode ser limitada a canais específicos em seu editor.

**Política de notificação.** Os controles globais que mantêm as notificações
poucas: a janela de lote, o limite por canal por hora, detecção de
oscilação sob *Mais opções*, a URL pública usada para os links "Abrir no
DumbMonit", e a linha de **escalonamento** — *se ninguém reconhecer* dentro
de um prazo, *também avise* outro canal, uma vez. O prazo conta a partir do
momento em que a primeira mensagem realmente saiu, e reconhecer o alerta (ou
sua resolução) interrompe o salto. Veja
[Política de notificação](../alerting/notifications.md).

Links: `/alerts#notifications` abre a aba, `/alerts#notifications-policy`
rola até o painel de política. Os links antigos de Configurações
(`/settings#notifications`, `/settings#notifications-policy`) redirecionam
para aqui.

## Operadores e visualizadores {#operators-and-viewers}

Um **operador** lida com alertas sem tocar na configuração: eles
reconhecem, colocam em soneca, ignoram uma regra para um dispositivo,
programam ou excluem janelas de manutenção e limpam histórico resolvido.
Regras, canais e a política de notificação permanecem somente leitura para
eles, marcados *Operador — configuração é restrita a administradores*.
Ignorar é a única substituição por dispositivo que um operador pode fazer;
limites permanecem uma decisão de administrador, e o *Ignorar* ou *Parar de
ignorar* de um operador mantém qualquer limite que um administrador tenha
definido para esse dispositivo.

Um visualizador vê toda aba mas nenhum controle: a página mostra
*Visualizador — somente leitura* onde um administrador encontraria os
botões.
