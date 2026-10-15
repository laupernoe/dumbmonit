# Atualizar o DumbMonit {#update-dumbmonit}

O DumbMonit avisa-o quando existe uma versão mais recente e guia-o pela
atualização. Nunca instala nada por si próprio.

## Detetar uma versão nova {#spotting-a-new-version}

No máximo uma vez por dia, o servidor consulta a API pública de versões do
GitHub (`api.github.com/repos/laupernoe/dumbmonit/releases`) para obter a
última versão e compara-a com a versão que utiliza. Pré-versões como
`0.1.0-alpha.7` são comparadas corretamente: `alpha.10` é mais recente do que
`alpha.9`, e `0.1.0` é mais recente do que qualquer `0.1.0-alpha.N`. Os
rascunhos são ignorados, e a uma instância numa versão estável nunca é
proposta uma pré-versão.

Quando existe uma versão mais recente, **Update available: vX** aparece no
canto inferior direito de todas as páginas e em **Settings → About**, onde
**How to update** abre os passos abaixo e **See what's new** reabre a janela
*What's new*. **Check now** verifica de novo imediatamente (no máximo uma vez
por minuto).

!!! note "O que é enviado"
    Um pedido `GET` anónimo com um agente de utilizador `DumbMonit/<versão>`.
    Nada sobre a sua instância, os seus monitores ou a sua rede é enviado,
    e o endereço está fixo no código.

### Desativar a verificação {#turning-the-check-off}

Para redes isoladas (air-gapped), desative **Check for new versions** em
**Settings → About** (administradores), ou defina a variável de ambiente, que
prevalece sobre o interruptor:

```yaml
environment:
  DUMBMONIT_UPDATE_CHECK: "off"
```

A mesma informação está disponível para scripts e tokens de API em
`GET /api/update`.

## Atualizar {#updating}

1. **Faça uma cópia de segurança.** Descarregue uma cópia de segurança em
   **Settings → Backup**, e guarde uma cópia de `/data/secret.key` (ver
   [Cópia de segurança e restauro](backup.md)). Sem essa chave, as senhas e
   tokens guardados não podem ser desencriptados.
2. **Obtenha a imagem e reinicie**, a partir da pasta que contém o seu
   `docker-compose.yml`:

    ```bash
    docker compose pull && docker compose up -d
    ```

    Os seus dados vivem no volume `/data` e são mantidos; as migrações da
    base de dados são executadas no arranque.
3. **Recarregue a página.** **Settings → About** mostra a nova versão e
   *You are up to date*.

Para fixar uma versão em vez de seguir `:latest`, altere a etiqueta da
imagem no ficheiro Compose (por exemplo `ghcr.io/laupernoe/dumbmonit:0.1.0-alpha.7`)
e execute o mesmo comando.

Para recuar, restaure a cópia de segurança feita no passo 1 na imagem anterior.
