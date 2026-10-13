# 网站健康和SEO {#website-health--seo}

通过监控网站，确保每个网站正常运行：当网站出现问题时（错误、无法访问、证书无效、重定向循环）、页面**空白化**、网站**从搜索引擎中消失** (`noindex`、`robots.txt`阻止所有访问）以及Google显示的元数据**发生变化**（标题、描述、规范URL、最终地址）时，您会收到通知。每个网站也根据它运行的平台（WordPress、Shopify、Wix、Next.js等）进行分类。

关于网站页面内容变化而非其健康状况，请参阅[网站变化](webchange.md)。

## 设置 {#set-it-up}

1. 在地址栏中粘贴要检查的页面，通常是主页，带或不带https://都可以。重定向会被跟随：http到https和example.com到www.example.com是正常的，只有重定向循环或移到其他地址才会被报告。
2. 可选地在"预期文本"中填写页面必须始终显示的短语，如果页面目的上比较短，调整"空白页面下方的字符数"。
3. 第一次检查会记录参考数据：最终地址、标题、描述、规范URL、索引规则。只有当连续两次检查都发现变化时，才会报告变化，因此A/B测试和易变的缓存会保持安静。
4. 要一次性添加许多网站，从设备列表导入CSV文件：每个网站都会归在 `Websites/<platform>` （WordPress、Shopify等），除非您指定其他文件夹。

!!! warning
    仅读取服务器发送的内容：完全由JavaScript绘制的页面在这里看起来是空白的。对于这样的网站，将"空白页面下方的字符数"设置为0。

## 每次检查做什么 {#what-each-check-does}

每个轮询间隔（默认60秒），页面会被读取一次：

- **重定向被逐个跟随**，最多**最大重定向次数**（默认10次），链条被记录：每个地址及其状态。从 `http://` 到 `https://` 或从 `example.com` 到 `www.example.com` 是大多数网站做的，永远不是错误。重定向回到已经看过的地址是一个**循环**，允许的重定向超过限制也是错误的：两者都会导致检查失败，消息中包含链条，就像网站不回答一样。
- **最终答案**：其HTTP状态、总时间（包括重定向）和TLS证书，验证方式与浏览器相同。拒绝的证书导致检查失败。
- **页面的可见文本**，如[网站变化](webchange.md)中所示：脚本、样式和隐藏块被排除。低于**空白页面下方的字符数**（默认20个，所以短合法页面通过）时页面算作空白。使用**预期文本**时，该短语必须出现在其中（大小写和间距被忽略）。
- **最终页面的SEO元数据**：`<title>`、元描述、规范URL、robots元标签和 `X-Robots-Tag` 标题（`noindex`、`nofollow`、`none`）、`og:title`、`og:description`、`<html lang>`、`<h1>` 的数量和 `hreflang` 替代物的数量。
- **每小时一次**，最终网站的 `robots.txt`：它是否禁止Googlebot（或 `*`）爬取 `/`？它公布的站点地图，或 `/sitemap.xml`：它是否能回答和被正确识别为站点地图？

用户代理是 `DumbMonit/<version>`，除非**用户代理**说明其他。

## 变化，确认两次 {#changes-confirmed-twice}

最终地址和元数据与参考进行比较。不同的值如果只看过一次是**待定的**；当下一次检查再次看到它时，它才成为变化。因此以两个版本（A/B测试）、犹豫的缓存或被截断的答案提供的页面保持安静。值的比较时忽略间距和大小写；地址忽略方案、前导 `www.`、末尾 `/` 和，使用**忽略查询字符串**（默认开启）时，URL参数如 `?utm_source=…`。

确认的变化保留在设备的历史中（字段、之前、之后、日期），其仪表盘 （`dumbmonit_website_title_changed` 等）保持为1达24小时。第一次检查仅记录参考。改变地址或**忽略查询字符串**会开始一个新参考，而不是报告变化。

## 选项 {#options}

| 选项 | 默认 | 含义 |
|---|---|---|
| 预期文本 | 空 | 页面必须显示的短语。 |
| 空白页面下方（字符） | 20 | 可见文本短于此为空白页面；0关闭检查。 |
| 最大重定向 | 10 | 最多跟随重定向（0到30）。 |
| 用户代理 | `DumbMonit/<version>` | 发送而非DumbMonit自己的。 |
| 忽略查询字符串 | 开 | 比较地址时URL参数不算。 |
| 允许回环和链路本地目标 | 关 | 用于由DumbMonit主机本身提供的网站。 |

## 平台检测 {#platform-detection}

平台来自以下位置（按顺序）：`<meta name="generator">` 标签、特定于主机或引擎的标头（`x-shopify-*`、`x-wix-*`、`x-drupal-cache`、`x-powered-by: Next.js` 等）、标记中的痕迹（`/wp-content/`、`cdn.shopify.com`、`data-wf-site`、`__NEXT_DATA__`、`/_nuxt/` 等）以及最后主机本身（`*.myshopify.com`、`*.wixsite.com` 等）。识别的：WordPress、Shopify、Wix、Squarespace、Webflow、Ghost、Drupal、Joomla、PrestaShop、Magento、Next.js、Nuxt、Gatsby、Hugo、Jekyll；其他都是**其他**。平台是 `dumbmonit_website_info` 的 `platform` 标签。

## 导入网站列表（CSV） {#import-a-list-of-sites-csv}

从设备列表或添加设备页面，**导入网站（CSV）**接受一个文件或最多有1000行的粘贴文本，其中包含标题行：

| 列 | 必需 | 含义 |
|---|---|---|
| `url` | 是 | 网站的地址。 |
| `name` | 否 | 设备名称；默认为主机名。 |
| `folder` | 否 | 设备的文件夹；空：`Websites/<platform>`，通过读取网站一次检测。 |
| `tags` | 否 | `key=value;key=value`。 |
| `interval` | 否 | 轮询间隔。 |
| `expect_text` | 否 | 与选项相同。 |

```csv
url,name,folder,tags,interval,expect_text
https://example.com,Example,,client=acme,60s,Example Inc.
https://shop.example.org,Shop,Customers/Acme,,300s,Add to cart
```

预览显示每一行及其错误，然后创建任何内容；已经监控的网站（相同的最终地址）被跳过。

## 指标 {#metrics}

| 指标 | 含义 |
|---|---|
| `dumbmonit_website_up` | 最终答案成功时为1（2xx或3xx），否则为0。 |
| `dumbmonit_website_status_code` | 最终答案的HTTP状态。 |
| `dumbmonit_website_response_seconds` | 总时间，包括重定向。 |
| `dumbmonit_website_redirect_hops` | 跟随的重定向。 |
| `dumbmonit_website_info{platform}` | 总为1；标签携带平台。 |
| `dumbmonit_website_text_chars` | 可见文本的字符数（2xx答案）。 |
| `dumbmonit_website_blank` | 当文本低于阈值时为1。 |
| `dumbmonit_website_expect_text_missing` | 当预期文本缺失时为1（设置了选项）。 |
| `dumbmonit_website_noindex` / `dumbmonit_website_nofollow` | 索引指令。 |
| `dumbmonit_website_h1_count` / `dumbmonit_website_hreflang_count` | `<h1>` 标题、`hreflang` 替代物。 |
| `dumbmonit_website_title_changed`、`…_description_changed`、`…_canonical_changed`、`…_final_url_changed` | 确认变化后24小时内为1。 |
| `dumbmonit_website_robots_blocks_all` | 当 `robots.txt` 禁止所有内容到Googlebot时为1。 |
| `dumbmonit_website_sitemap_ok` | 当站点地图回答和被正确识别时为1。 |

## 内置警报 {#built-in-alerts}

| 警报 | 何时 | 严重程度 |
|---|---|---|
| 设备无法到达 | 无答案、拒绝证书、重定向循环或太多重定向 | 严重 |
| 网站关闭 | 最终答案是4xx或5xx错误，持续3分钟 | 严重 |
| 网站页面空白 | 可见文本低于阈值，持续3分钟 | 严重 |
| 网站预期文本缺失 | 预期文本缺失，持续3分钟 | 警告 |
| 网站不可索引 | 页面上的 `noindex`，持续10分钟 | 严重 |
| 网站被robots.txt阻止 | `robots.txt` 禁止所有内容，持续10分钟 | 严重 |
| 网站规范URL已更改 | 确认变化 | 警告 |
| 网站地址已更改 | 最终主机或路径已更改，已确认 | 警告 |
| 网站标题或描述已更改 | 确认变化 | 信息 |
| 网站缓慢 | 超过5秒，持续10分钟 | 警告 |

检查在DumbMonit服务器本身上运行：它不能通过中继代理。
