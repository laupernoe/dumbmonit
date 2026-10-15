# Executar uma instância de demonstração {#run-a-demo-instance}

A demonstração pública em **[demo.dumbmonit.app](https://demo.dumbmonit.app)**
é uma imagem DumbMonit normal, iniciada com `DUMBMONIT_DEMO=1`. Pode executar
a mesma coisa — para mostrar o DumbMonit a uma equipa antes de o instalar para
valer, ou para colocar uma vitrine no seu próprio site.

!!! warning "Nunca num volume de dados real"
    O modo de demonstração **recria a base de dados a cada arranque**.
    Dê-lhe um volume próprio, ou nenhum: apontado para o `/data` de uma
    instância real, substituiria os seus monitores, regras e histórico
    pelo património fictício.

## O que o modo de demonstração faz {#what-demo-mode-does}

- **Iniciar sessão com `demo` / `demo`.** A página de início de sessão diz
  isso mesmo e preenche os campos com um clique. A conta vê todos os ecrãs que
  um administrador vê.
- **Só leitura.** Todas as alterações — monitores, regras, silêncios,
  confirmações, canais, tokens, utilizadores, definições, cópias de segurança
  e restauros, análises de descoberta, registo de agentes, ferramentas de
  escrita do endpoint do assistente — são recusadas pelo servidor com `403` e
  `{"error": "This is a read-only demo…", "demo": true}`. Iniciar e terminar
  sessão são as únicas escritas permitidas. A interface mostra a mensagem no
  local onde o clique aconteceu.
- **Um património fixo e fictício.** No arranque, a base de dados é criada
  do zero e semeada com um pequeno homelab sob `*.home.arpa`: um cluster
  Proxmox VE, um NAS Synology, um Proxmox Backup Server, um TrueNAS, uma
  firewall OPNsense, um servidor Redfish com uma peça em falha, algumas
  verificações HTTP, TLS e ping, um anfitrião com agente e uma página de
  estado. Os monitores são respondidos por simuladores falsos dentro do
  servidor, ligados a `127.0.0.1` e a reproduzir respostas de API gravadas,
  pelo que os coletores reais e os painéis de monitor funcionam. Reiniciar
  devolve o mesmo património.
- **Sete dias de histórico.** Métricas sintéticas são escritas na
  VictoriaMetrics no arranque, para que os gráficos, *Últimos 7 dias*, as
  previsões e o histórico de alertas estejam preenchidos, e alguns alertas
  estão ativos para que *Precisa de si* tenha algo.
- **Nada sai do servidor.** Nenhuma notificação é enviada, e nenhuma
  verificação vai a um endereço que um visitante tenha escrito.
- **Uma visita guiada** abre após a primeira sessão iniciada; a faixa reabre-a.

A limitação de taxa do início de sessão funciona como habitualmente. Coloque a
demonstração atrás do seu proxy inverso habitual com HTTPS e defina
`DUMBMONIT_COOKIE_SECURE=1` e `DUMBMONIT_TRUSTED_PROXIES` como para qualquer
instância pública.

## Iniciar {#start-it}

```bash
docker run -d --name dumbmonit-demo -p 8080:8080 \
  -e DUMBMONIT_DEMO=1 \
  ghcr.io/laupernoe/dumbmonit:latest
```

Ou com Compose, sem volume para que cada reinício comece limpo:

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

Obtenha a imagem regularmente (ou deixe uma ferramenta como o Watchtower
fazê-lo): a demonstração mostra então sempre a última versão com os mesmos
dados.
