# Executar uma instância de demonstração {#run-a-demo-instance}

A demo pública em **[demo.dumbmonit.app](https://demo.dumbmonit.app)** é uma
imagem comum do DumbMonit iniciada com `DUMBMONIT_DEMO=1`. Você pode executar a
mesma coisa — para mostrar o DumbMonit a uma equipe antes de instalá-lo de
verdade, ou para colocar uma vitrine no seu próprio site.

!!! warning "Nunca em um volume de dados real"
    O modo demo **recria o banco de dados a cada inicialização**. Dê a ele um
    volume próprio, ou nenhum: apontado para o `/data` de uma instância real,
    ele substituiria seus monitores, regras e histórico pelo acervo fictício.

## O que o modo demo faz {#what-demo-mode-does}

- **Entre com `demo` / `demo`.** A página de login avisa isso e preenche os
  campos com um clique. A conta vê todas as telas que um administrador vê.
- **Somente leitura.** Toda alteração — monitores, regras, silêncios,
  reconhecimentos, canais, tokens, usuários, configurações, backups e
  restaurações, varreduras de descoberta, cadastro de agentes, ferramentas de
  escrita do endpoint do assistente — é recusada pelo servidor com `403` e
  `{"error": "This is a read-only demo…", "demo": true}`. Entrar e sair são as
  únicas escritas permitidas. A interface mostra a mensagem onde o clique
  aconteceu.
- **Um acervo fixo e fictício.** Na inicialização o banco de dados é criado do
  zero e populado com um pequeno homelab sob `*.home.arpa`: um cluster Proxmox
  VE, um NAS Synology, um Proxmox Backup Server, um TrueNAS, um firewall
  OPNsense, um servidor Redfish com uma peça falhando, algumas verificações
  HTTP, TLS e ping, um host com agente e uma página de status. Os monitores
  são respondidos por simuladores falsos dentro do servidor, vinculados a
  `127.0.0.1` e reproduzindo respostas de API gravadas, então os coletores e
  painéis de monitor reais são executados. Reiniciar entrega o mesmo
  acervo.
- **Trinta dias de histórico.** Métricas sintéticas são gravadas no
  VictoriaMetrics na inicialização (a última semana com resolução de cinco
  minutos), então gráficos, *Últimos 7 dias*, disponibilidade em 30 dias,
  previsões ("cheio em cerca de N dias") e histórico de alertas ficam
  populados, e alguns alertas estão disparando para que *Precisa de você*
  tenha algo. Um switch central com suas portas e seu tráfego, um mapa de
  ameaças alimentado por contagens inventadas do CrowdSec e da Cloudflare,
  vencimentos de domínios e licenças, uma matriz de backups com algumas noites
  com falha, um dispositivo encontrado pela varredura de rede e duas páginas de
  status com cenas noturnas completam o quadro. Tudo é calculado em memória a
  partir de uma semente fixa: nada é buscado na rede.
- **Nada sai do servidor.** Nenhuma notificação é enviada, e nenhuma sondagem
  vai a um endereço que um visitante tenha digitado.
- **Um tour guiado** abre após o primeiro login; o banner o reabre.

A limitação de taxa do login funciona normalmente. Coloque a demo detrás do
seu proxy reverso habitual com HTTPS e defina `DUMBMONIT_COOKIE_SECURE=1` e
`DUMBMONIT_TRUSTED_PROXIES` como em qualquer instância pública.

## Inicie-a {#start-it}

```bash
docker run -d --name dumbmonit-demo -p 8080:8080 \
  -e DUMBMONIT_DEMO=1 \
  ghcr.io/laupernoe/dumbmonit:latest
```

Ou com Compose, sem volume, para que cada reinício comece do zero:

```yaml
services:
  dumbmonit-demo:
    image: ghcr.io/laupernoe/dumbmonit:latest
    restart: unless-stopped
    ports:
      - "8080:8080"
    environment:
      DUMBMONIT_DEMO: "1"
```

Baixe a imagem regularmente (ou deixe uma ferramenta como o Watchtower fazer
isso): a demo então sempre mostra o lançamento mais recente com os mesmos
dados.
