# A página de visão geral {#the-overview-page}

A página inicial responde "tudo está bem?" em cerca de um segundo, depois
deixa você se aprofundar quando quiser.

![A visão geral](../assets/screenshots/overview-light.png){ loading=lazy }

## O boletim {#the-bulletin}

A faixa no topo é um boletim meteorológico para sua rede. Uma frase declara
o céu: "Céu limpo." quando nada precisa de você, senão uma contagem do que
precisa ("9 avisos, 1 inacessível."). Abaixo dela, placas contam dispositivos
reportando e inacessíveis, com o horário da última verificação, e três
leituras: **Reportando**, **Precisa de você**, **Previsões**.

A janela do tempo à direita acompanha a rede: limpo, nublado, carregado ou
tempestade, dependendo de quantos alertas estão disparando e quão severos
são, com o pombo voando pela tela. Sob `prefers-reduced-motion`, tudo fica
imóvel.

A única ação principal da página é **Adicionar dispositivo**.

## O guia de primeira execução {#the-first-run-guide}

Em uma instância nova, a página abaixo do boletim não está vazia: é um guia
de três passos — adicione um dispositivo, conecte uma forma de ser avisado,
verifique que uma mensagem chega. Cada passo é um clique, cada um pode ser
dispensado por conta própria, e **Pular o guia** afasta todos os três. A
escolha é armazenada na instância, então um segundo navegador não reinicia
o guia. Veja
[Adicione seu primeiro dispositivo](../install/first-device.md#the-three-first-steps).

## Precisa de você {#needs-you}

A lista do que precisa de você agora, mais severo primeiro: dispositivos
inacessíveis, depois alertas disparando por severidade. Cada linha nomeia a
regra, o dispositivo, o valor atual, há quanto tempo está acontecendo, e se
agrupa várias séries ("×4" para quatro sistemas de arquivos no mesmo
dispositivo). Quatro ações: **Reconhecer** (reconhecer este alerta por 1 h,
4 h, 24 h ou até resolver — lembretes pausam, a resolução ainda é
notificada), **Soneca** (ocultar apenas este alerta por 1 h, 8 h, 1 dia ou
até resolver), **Ignorar** (parar esta regra de jamais alertar neste
dispositivo de novo, reversivelmente), e **Abrir dispositivo**.

Alertas reconhecidos e em soneca se movem para seu próprio grupo mais
silencioso sob a lista e saem da contagem de *Precisa de você*. Um alerta
ignorado desaparece de vez — não é "conhecido", é desligado para esse
dispositivo. Veja
[Se livrando de um alerta](alerts.md#getting-rid-of-an-alert).

## Resumo de dispositivos {#devices-summary}

Uma linha: quantos dispositivos, quantos reportando, quantos inacessíveis, e
um link para a página completa de Dispositivos. A visão geral nunca lista
dispositivos: o rack vive em [Dispositivos](devices.md).

## Previsões e últimas 24 horas {#forecasts-and-last-24-hours}

**Previsões** mostra o que é previsto em vez de observado: a anomalia de
linha de base sazonal, as extrapolações "cheio em quatro dias", a estimativa
de esgotamento do PBS, e regras cujo nome anuncia uma previsão. Enquanto uma
linha de base está aprendendo, suas entradas avisam isso.

**Últimas 24 horas** lista as transições disparadas do dia, com um link para
o histórico completo na página de Alertas.

## Modo parede {#wall-mode}

`/wall` mostra apenas o boletim, em tela cheia, para um monitor de ambiente:
a frase, as contagens e a lista *Precisa de você*, atualizados a cada 20
segundos, com a tela mantida acordada. ++esc++ ou **Sair** volta à visão
geral. Abra-o a partir da paleta de comandos ("Modo parede") ou digitando a
URL. Ele também pode tocar música iniciada do seu celular: veja
[Modo parede e música](wall.md).

![Modo parede](../assets/screenshots/wall-light.png){ loading=lazy }

## Instale-o em um celular {#install-it-on-a-phone}

A interface entrega um manifesto web, então um celular pode adicioná-la à
tela inicial: **Compartilhar → Adicionar à Tela de Início** no iOS,
**Instalar aplicativo** no menu do Chrome no Android. Ela então abre sem a
moldura do navegador, no tema para o qual o celular está configurado.

Deliberadamente não há **modo offline e nenhum service worker**. Uma tela de
monitoramento que mostra uma página em cache é pior que uma que diz não
conseguir alcançar o servidor: o ponto principal é que o que você lê está
atual. Sem um worker, um servidor atualizado também é servido imediatamente,
sem build desatualizado para limpar.

## Paleta de comandos {#command-palette}

++cmd+k++ no macOS, ++ctrl+k++ nos demais, abre a paleta de comandos: um
campo, resultados em três grupos.

| Grupo | Entradas |
|---|---|
| Páginas | Visão geral, Dispositivos, Alertas, Notificações, Páginas de status, Configurações, Modo parede, Documentação |
| Ações | Adicionar dispositivo, Varrer minha rede, Programar manutenção, Adicionar canal de notificação, Nova página de status, Anunciar um incidente, Alternar tema |
| Dispositivos | Todo dispositivo, com seu LED de estado; ++enter++ o abre |

## Navegação {#navigation}

A barra superior tem Visão geral, Monitores, Alertas (com a contagem do que
precisa de você) e Configurações, o atalho da paleta e o alternador de tema.
Em um celular, ela se torna uma barra de abas inferior com essas quatro abas
e **Mais**, que guarda o modo parede e a documentação.

- **Visão geral** — o boletim: tudo está bem, e o que precisa de você.
- **Monitores** — o rack; toda fonte que você monitora. Veja
  [Página de dispositivos](devices.md).
- **Alertas** — o que está disparando, as janelas de manutenção, o histórico
  e as regras. Veja [Página de alertas](alerts.md).
- **Configurações** — sua conta, usuários e login único, tokens de agente e
  assistente, canais de notificação e relatórios, páginas de status públicas,
  aparência, sobre. Veja [Configurações](settings.md). Nada na
  interface transmite status apenas por cor: uma placa ou um LED sempre vem
  com uma palavra (Reportando, Inacessível, Aviso, Alerta, Suprimido pelo
  pai, Acumulando).
