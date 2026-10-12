# Banner para o site

Mostre um banner no seu próprio site quando serviços estiverem fora do ar ("2
serviços estão fora do ar"), alimentado por uma [página de status](../using/status-pages.md)
publicada. Quando tudo está bem, o banner fica oculto, a menos que você peça
para ele ser mostrado sempre ("Todos os sistemas operacionais").

Três formas de adicioná-lo, da mais simples à mais controlada. Arquivos prontos
para copiar estão na pasta `integrations/` do repositório.

## Trecho HTML

Cole isto antes de `</body>`, com o endereço do seu servidor e o endereço da página:

```html
<script async
  src="https://monit.example.com/api/public/status/my-page/banner.js"
  data-position="top" data-show="issues" data-lang="en"></script>
```

| Atributo | Valores | Padrão |
|---|---|---|
| `data-position` | `top`, `bottom` | `top` |
| `data-show` | `issues` (só quando algo está errado), `always` | `issues` |
| `data-lang` | `en`, `fr`, `de`, `es`, `it`, `pt`, `pt-BR`, `ru`, `zh-Hans` | idioma da página, senão `en` |

O script não tem dependências, desenha dentro de um Shadow DOM para que seus
estilos não o afetem, segue a preferência de claro ou escuro do visitante e
pode ser descartado (até a aba do navegador ser fechada). Ele lê o JSON público
da página, que o servidor permite que qualquer site busque
(`Access-Control-Allow-Origin: *`, sem cookies). Apenas o documento público da
página de status é aberto desse jeito: o resto da API mantém suas regras.

## WordPress

Copie `integrations/wordpress/dumbmonit-status-banner` para
`wp-content/plugins/`, ative-o, depois abra **Settings > DumbMonit Banner** e
informe a URL da página de status
(`https://monit.example.com/api/public/status/my-page`), a posição e o modo de
exibição. O plugin busca o JSON no servidor, faz cache por um minuto e escapa
tudo o que imprime. O WordPress recusa requisições a endereços privados, então
o servidor DumbMonit precisa estar acessível em um endereço público.

## Include PHP

Para qualquer site PHP, sem framework:

```php
require __DIR__ . '/banner.php';
dumbmonit_banner('https://monit.example.com/api/public/status/my-page',
    ['position' => 'top', 'show' => 'issues', 'ttl' => 60]);
```

O helper aceita apenas URLs `http` e `https`, usa um tempo limite de 3
segundos, não segue redirecionamentos, mantém um cache em arquivo no diretório
temporário e não imprime nada se o servidor não puder ser alcançado. Veja
`integrations/php/example.php`.
