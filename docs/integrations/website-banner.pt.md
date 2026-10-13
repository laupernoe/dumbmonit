# Banner para o seu site {#website-banner}

Mostre um banner no seu próprio site quando há serviços em baixo («2 serviços
estão em baixo»), alimentado por uma [página de estado](../using/status-pages.md)
publicada. Quando está tudo bem, o banner fica escondido, a menos que peça que
seja mostrado («Todos os sistemas operacionais»).

Há três formas de o adicionar, da mais simples à mais controlada. Os ficheiros
prontos a copiar estão na pasta `integrations/` do repositório.

## Excerto HTML {#html-snippet}

Cole isto antes de `</body>`, com o endereço do seu servidor e o endereço da página:

```html
<script async
  src="https://monit.example.com/api/public/status/my-page/banner.js"
  data-position="top" data-show="issues" data-lang="en"></script>
```

| Atributo | Valores | Predefinição |
|---|---|---|
| `data-position` | `top`, `bottom` | `top` |
| `data-show` | `issues` (só quando algo está errado), `always` | `issues` |
| `data-lang` | `en`, `fr`, `de`, `es`, `it`, `pt`, `pt-BR`, `ru`, `zh-Hans` | idioma da página, senão `en` |

O script não tem dependências, desenha dentro de um Shadow DOM para que os seus
estilos não lhe toquem, segue a preferência clara ou escura do visitante e pode
ser dispensado (até o separador do navegador ser fechado). Lê o JSON público da
página, que o servidor permite que qualquer site obtenha
(`Access-Control-Allow-Origin: *`, sem cookies). Só o documento público da
página de estado é aberto desta forma; o resto da API mantém as suas regras.

## WordPress {#wordpress}

Copie `integrations/wordpress/dumbmonit-status-banner` para
`wp-content/plugins/`, ative-o, depois abra **Definições > DumbMonit Banner**
e introduza o URL da página de estado
(`https://monit.example.com/api/public/status/my-page`), a posição e o modo de
apresentação. O plugin obtém o JSON no servidor, guarda-o em cache durante um
minuto e escapa tudo o que imprime.

O servidor DumbMonit pode estar num endereço privado (LAN): o plugin só pede
sempre o URL exato que guardou (verificado contra o padrão
`/api/public/status/<address>`, redirecionamentos desativados). Se o endereço
for inválido, a página de definições mostra um erro e mantém o valor anterior.
Se a página de estado não puder ser obtida, é mostrado um aviso na
administração do WordPress (e ao guardar), em vez de falhar silenciosamente.

O texto do banner é traduzível (domínio de texto `dumbmonit-status-banner`,
modelo em `languages/`); segue o idioma do site WordPress. O JSON público do
estado não traz idioma, por isso não pode escolher um.

## Include PHP {#php-include}

Para qualquer site PHP, sem framework:

```php
require __DIR__ . '/banner.php';
dumbmonit_banner('https://monit.example.com/api/public/status/my-page',
    ['position' => 'top', 'show' => 'issues', 'ttl' => 60]);
```

O auxiliar aceita apenas URLs `http` e `https`, usa um tempo limite de 3
segundos, não segue redirecionamentos, mantém uma cache de ficheiros na pasta
temporária e não imprime nada se o servidor não puder ser alcançado (a falha é
escrita no registo de erros do PHP). Os textos estão em inglês por predefinição;
passe `'lang' => 'fr'` ou uma matriz `'strings'` para os alterar. Um endereço
privado (LAN) funciona. Veja `integrations/php/example.php`.
