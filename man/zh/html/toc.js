// Populate the sidebar
//
// This is a script, and not included directly in the page, to control the total size of the book.
// The TOC contains an entry for each page, so if each page includes a copy of the TOC,
// the total size of the page becomes O(n**2).
class MDBookSidebarScrollbox extends HTMLElement {
    constructor() {
        super();
    }
    connectedCallback() {
        this.innerHTML = '<ol class="chapter"><li class="chapter-item expanded "><a href="说明.html"><strong aria-hidden="true">1.</strong> 说明</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="特性.html"><strong aria-hidden="true">1.1.</strong> 特性</a></li><li class="chapter-item expanded "><a href="安装.html"><strong aria-hidden="true">1.2.</strong> 安装</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="从源码构建.html"><strong aria-hidden="true">1.2.1.</strong> 从源码构建</a></li><li class="chapter-item expanded "><a href="环境要求.html"><strong aria-hidden="true">1.2.2.</strong> 环境要求</a></li><li class="chapter-item expanded "><a href="预编译二进制.html"><strong aria-hidden="true">1.2.3.</strong> 预编译二进制</a></li></ol></li><li class="chapter-item expanded "><a href="快速开始.html"><strong aria-hidden="true">1.3.</strong> 快速开始</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="1-配置源.html"><strong aria-hidden="true">1.3.1.</strong> 1. 配置源</a></li><li class="chapter-item expanded "><a href="2-查询可用技能.html"><strong aria-hidden="true">1.3.2.</strong> 2. 查询可用技能</a></li><li class="chapter-item expanded "><a href="3-安装技能.html"><strong aria-hidden="true">1.3.3.</strong> 3. 安装技能</a></li><li class="chapter-item expanded "><a href="4-列出已安装技能.html"><strong aria-hidden="true">1.3.4.</strong> 4. 列出已安装技能</a></li><li class="chapter-item expanded "><a href="5-移除技能.html"><strong aria-hidden="true">1.3.5.</strong> 5. 移除技能</a></li></ol></li><li class="chapter-item expanded "><a href="命令.html"><strong aria-hidden="true">1.4.</strong> 命令</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="sources-—-管理配置源.html"><strong aria-hidden="true">1.4.1.</strong> sources — 管理配置源</a></li><li class="chapter-item expanded "><a href="platforms-—-列出配置平台.html"><strong aria-hidden="true">1.4.2.</strong> platforms — 列出配置平台</a></li><li class="chapter-item expanded "><a href="add-—-安装技能.html"><strong aria-hidden="true">1.4.3.</strong> add — 安装技能</a></li><li class="chapter-item expanded "><a href="remove-—-移除技能.html"><strong aria-hidden="true">1.4.4.</strong> remove — 移除技能</a></li><li class="chapter-item expanded "><a href="update-—-更新已安装技能.html"><strong aria-hidden="true">1.4.5.</strong> update — 更新已安装技能</a></li><li class="chapter-item expanded "><a href="restore-—-从锁文件恢复技能.html"><strong aria-hidden="true">1.4.6.</strong> restore — 从锁文件恢复技能</a></li><li class="chapter-item expanded "><a href="list-—-列出已安装技能.html"><strong aria-hidden="true">1.4.7.</strong> list — 列出已安装技能</a></li><li class="chapter-item expanded "><a href="query-—-查询源中的技能.html"><strong aria-hidden="true">1.4.8.</strong> query — 查询源中的技能</a></li><li class="chapter-item expanded "><a href="find-—-交互式查找并安装技能.html"><strong aria-hidden="true">1.4.9.</strong> find — 交互式查找并安装技能</a></li><li class="chapter-item expanded "><a href="rec-—-管理推荐技能.html"><strong aria-hidden="true">1.4.10.</strong> rec — 管理推荐技能</a></li><li class="chapter-item expanded "><a href="cache-—-管理技能缓存.html"><strong aria-hidden="true">1.4.11.</strong> cache — 管理技能缓存</a></li><li class="chapter-item expanded "><a href="config-—-管理配置.html"><strong aria-hidden="true">1.4.12.</strong> config — 管理配置</a></li><li class="chapter-item expanded "><a href="new-—-创建技能项目.html"><strong aria-hidden="true">1.4.13.</strong> new — 创建技能项目</a></li></ol></li><li class="chapter-item expanded "><a href="配置.html"><strong aria-hidden="true">1.5.</strong> 配置</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="配置文件位置.html"><strong aria-hidden="true">1.5.1.</strong> 配置文件位置</a></li><li class="chapter-item expanded "><a href="配置结构.html"><strong aria-hidden="true">1.5.2.</strong> 配置结构</a></li><li class="chapter-item expanded "><a href="完整示例.html"><strong aria-hidden="true">1.5.3.</strong> 完整示例</a></li><li class="chapter-item expanded "><a href="平台.html"><strong aria-hidden="true">1.5.4.</strong> 平台</a></li><li class="chapter-item expanded "><a href="源.html"><strong aria-hidden="true">1.5.5.</strong> 源</a></li><li class="chapter-item expanded "><a href="推荐.html"><strong aria-hidden="true">1.5.6.</strong> 推荐</a></li><li class="chapter-item expanded "><a href="缓存.html"><strong aria-hidden="true">1.5.7.</strong> 缓存</a></li><li class="chapter-item expanded "><a href="注册中心.html"><strong aria-hidden="true">1.5.8.</strong> 注册中心</a></li><li class="chapter-item expanded "><a href="--from-参数解析.html"><strong aria-hidden="true">1.5.9.</strong> --from 参数解析</a></li><li class="chapter-item expanded "><a href="--skill-参数.html"><strong aria-hidden="true">1.5.10.</strong> --skill 参数</a></li><li class="chapter-item expanded "><a href="--agent-验证规则.html"><strong aria-hidden="true">1.5.11.</strong> --agent 验证规则</a></li><li class="chapter-item expanded "><a href="url-归一化.html"><strong aria-hidden="true">1.5.12.</strong> URL 归一化</a></li></ol></li><li class="chapter-item expanded "><a href="安装模型：规范目录-+-软链接.html"><strong aria-hidden="true">1.6.</strong> 安装模型：规范目录 + 软链接</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="全局-vs-本地路径.html"><strong aria-hidden="true">1.6.1.</strong> 全局 vs 本地路径</a></li><li class="chapter-item expanded "><a href="软链接规则.html"><strong aria-hidden="true">1.6.2.</strong> 软链接规则</a></li><li class="chapter-item expanded "><a href="回退机制.html"><strong aria-hidden="true">1.6.3.</strong> 回退机制</a></li><li class="chapter-item expanded "><a href="平台目录不存在时的行为.html"><strong aria-hidden="true">1.6.4.</strong> 平台目录不存在时的行为</a></li></ol></li><li class="chapter-item expanded "><a href="锁文件.html"><strong aria-hidden="true">1.7.</strong> 锁文件</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="位置.html"><strong aria-hidden="true">1.7.1.</strong> 位置</a></li><li class="chapter-item expanded "><a href="格式.html"><strong aria-hidden="true">1.7.2.</strong> 格式</a></li><li class="chapter-item expanded "><a href="条目字段.html"><strong aria-hidden="true">1.7.3.</strong> 条目字段</a></li><li class="chapter-item expanded "><a href="顶层字段.html"><strong aria-hidden="true">1.7.4.</strong> 顶层字段</a></li></ol></li><li class="chapter-item expanded "><a href="从-xskillyaml-迁移.html"><strong aria-hidden="true">1.8.</strong> 从 .xskill.yaml 迁移</a></li><li class="chapter-item expanded "><a href="编辑器集成.html"><strong aria-hidden="true">1.9.</strong> 编辑器集成</a></li><li class="chapter-item expanded "><a href="开发.html"><strong aria-hidden="true">1.10.</strong> 开发</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="构建.html"><strong aria-hidden="true">1.10.1.</strong> 构建</a></li><li class="chapter-item expanded "><a href="测试.html"><strong aria-hidden="true">1.10.2.</strong> 测试</a></li><li class="chapter-item expanded "><a href="项目结构.html"><strong aria-hidden="true">1.10.3.</strong> 项目结构</a></li></ol></li><li class="chapter-item expanded "><a href="许可证.html"><strong aria-hidden="true">1.11.</strong> 许可证</a></li></ol></li></ol>';
        // Set the current, active page, and reveal it if it's hidden
        let current_page = document.location.href.toString().split("#")[0].split("?")[0];
        if (current_page.endsWith("/")) {
            current_page += "index.html";
        }
        var links = Array.prototype.slice.call(this.querySelectorAll("a"));
        var l = links.length;
        for (var i = 0; i < l; ++i) {
            var link = links[i];
            var href = link.getAttribute("href");
            if (href && !href.startsWith("#") && !/^(?:[a-z+]+:)?\/\//.test(href)) {
                link.href = path_to_root + href;
            }
            // The "index" page is supposed to alias the first chapter in the book.
            if (link.href === current_page || (i === 0 && path_to_root === "" && current_page.endsWith("/index.html"))) {
                link.classList.add("active");
                var parent = link.parentElement;
                if (parent && parent.classList.contains("chapter-item")) {
                    parent.classList.add("expanded");
                }
                while (parent) {
                    if (parent.tagName === "LI" && parent.previousElementSibling) {
                        if (parent.previousElementSibling.classList.contains("chapter-item")) {
                            parent.previousElementSibling.classList.add("expanded");
                        }
                    }
                    parent = parent.parentElement;
                }
            }
        }
        // Track and set sidebar scroll position
        this.addEventListener('click', function(e) {
            if (e.target.tagName === 'A') {
                sessionStorage.setItem('sidebar-scroll', this.scrollTop);
            }
        }, { passive: true });
        var sidebarScrollTop = sessionStorage.getItem('sidebar-scroll');
        sessionStorage.removeItem('sidebar-scroll');
        if (sidebarScrollTop) {
            // preserve sidebar scroll position when navigating via links within sidebar
            this.scrollTop = sidebarScrollTop;
        } else {
            // scroll sidebar to current active section when navigating via "next/previous chapter" buttons
            var activeSection = document.querySelector('#sidebar .active');
            if (activeSection) {
                activeSection.scrollIntoView({ block: 'center' });
            }
        }
        // Toggle buttons
        var sidebarAnchorToggles = document.querySelectorAll('#sidebar a.toggle');
        function toggleSection(ev) {
            ev.currentTarget.parentElement.classList.toggle('expanded');
        }
        Array.from(sidebarAnchorToggles).forEach(function (el) {
            el.addEventListener('click', toggleSection);
        });
    }
}
window.customElements.define("mdbook-sidebar-scrollbox", MDBookSidebarScrollbox);
