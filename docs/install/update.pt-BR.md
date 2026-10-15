# Atualizar o DumbMonit {#update-dumbmonit}

O DumbMonit avisa quando existe uma versão mais nova e orienta você durante a
atualização. Ele nunca instala nada por conta própria.

## Detectando uma nova versão {#spotting-a-new-version}

No máximo uma vez por dia, o servidor consulta a API pública de releases do
GitHub (`api.github.com/repos/laupernoe/dumbmonit/releases`) pela versão mais
recente e a compara com a que você usa. Pré-lançamentos como `0.1.0-alpha.7`
são comparados corretamente: `alpha.10` é mais novo que `alpha.9`, e `0.1.0` é
mais novo que qualquer `0.1.0-alpha.N`. Rascunhos são ignorados, e uma
instância em uma versão estável nunca recebe oferta de um pré-lançamento.

Quando existe uma versão mais nova, **Atualização disponível: vX** aparece no
canto inferior direito de toda página e em **Configurações → Sobre**, onde
**Como atualizar** abre os passos abaixo e **Ver novidades** reabre a janela
*O que há de novo*. **Verificar agora** checa de novo imediatamente (no máximo
uma vez por minuto).

!!! note "O que é enviado"
    Uma requisição `GET` anônima com um user agent `DumbMonit/<version>`. Nada
    sobre sua instância, seus monitores ou sua rede é enviado, e o endereço
    é fixo no código.

### Desativando a verificação {#turning-the-check-off}

Para redes isoladas (air-gapped), desligue **Verificar novas versões** em
**Configurações → Sobre** (administradores), ou defina a variável de
ambiente, que tem prioridade sobre o botão:

```yaml
environment:
  DUMBMONIT_UPDATE_CHECK: "off"
```

A mesma informação está disponível para scripts e tokens de API em
`GET /api/update`.

## Atualizando {#updating}

1. **Faça backup.** Baixe um backup em **Configurações → Backup**, e guarde
   uma cópia de `/data/secret.key` (veja [Backup e restauração](backup.md)).
   Sem essa chave, senhas e tokens armazenados não podem ser descriptografados.
2. **Baixe e reinicie**, a partir da pasta que contém seu `docker-compose.yml`:

    ```bash
    docker compose pull && docker compose up -d
    ```

    Seus dados ficam no volume `/data` e são preservados; as migrações do
    banco de dados rodam ao iniciar.
3. **Recarregue a página.** **Configurações → Sobre** mostra a nova versão e
   *Você está atualizado*.

Para fixar uma versão em vez de seguir `:latest`, mude a tag da imagem no
arquivo Compose (por exemplo `ghcr.io/laupernoe/dumbmonit:0.1.0-alpha.7`) e
execute o mesmo comando.

Para voltar atrás, restaure o backup feito no passo 1 sobre a imagem anterior.
