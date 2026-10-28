# Modo parede {#wall-mode-and-music}

`/wall` mostra o quadro próprio do console, em tela cheia, para um monitor que
fica ligado em uma sala. Atualiza-se sozinho e mantém a tela acordada.

De cima para baixo:

- o **veredito** em poucas palavras ("All good", "3 problems") com a data e o
  relógio, e quando foi atualizado pela última vez;
- os **cinco números** da visão geral: monitores ativos, disponibilidade em 30
  dias, tempo de resposta p95, alertas abertos e tempo médio de resolução;
- o **rio de saúde**, a grande imagem do quadro: uma faixa por monitor, os piores
  no topo;
- os **alertas abertos**, os mais graves primeiro;
- **quem está de plantão** agora, com a escala e o fim do turno.

Sem música e sem widgets: fundo preto, a tipografia do console,
estados escritos junto de uma forma e nunca só por cor.

## Abrir e sair

Abra-o com o ícone **Wall mode** da barra superior, pela paleta de comandos
("Wall mode") ou digitando a URL. ++esc++ ou **Exit** volta ao console. Uma conta
limitada a algumas equipes ou clientes não tem o modo parede, porque ele mostra
toda a frota.

## Fundo {#backdrop}

A parede compartilha a aparência escura do console. O botão **Fundo** (no alto
à direita, ao lado de Exit) pode desenhar uma das cidades à noite das
[páginas de status](status-pages.md) atrás do veredito e do relógio: Paris,
Nova York, Tóquio ou Dubai, ou *Auto* para a mais próxima do fuso horário do
navegador. Ele vem desligado e é lembrado por navegador; `?city=paris` (ou
`newyork`, `tokyo`, `dubai`, `none`) em um link escolhe só para aquela tela. O
céu acompanha o painel: estrelas quando está tudo bem, nuvens com um aviso,
tempestade com um alerta. No celular, ou com a aba oculta, a cidade fica
parada. Um `zermatt` salvo ou em um link agora cai em Dubai.
