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
        this.innerHTML = '<ol class="chapter"><li class="chapter-item expanded "><a href="introduction.html"><strong aria-hidden="true">1.</strong> Introduction</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="features.html"><strong aria-hidden="true">1.1.</strong> Features</a></li><li class="chapter-item expanded "><a href="installation.html"><strong aria-hidden="true">1.2.</strong> Installation</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="from-source.html"><strong aria-hidden="true">1.2.1.</strong> From source</a></li><li class="chapter-item expanded "><a href="requirements.html"><strong aria-hidden="true">1.2.2.</strong> Requirements</a></li><li class="chapter-item expanded "><a href="pre-built-binaries.html"><strong aria-hidden="true">1.2.3.</strong> Pre-built binaries</a></li></ol></li><li class="chapter-item expanded "><a href="quick-start.html"><strong aria-hidden="true">1.3.</strong> Quick Start</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="1-configure-a-source.html"><strong aria-hidden="true">1.3.1.</strong> 1. Configure a source</a></li><li class="chapter-item expanded "><a href="2-query-available-skills.html"><strong aria-hidden="true">1.3.2.</strong> 2. Query available skills</a></li><li class="chapter-item expanded "><a href="3-install-a-skill.html"><strong aria-hidden="true">1.3.3.</strong> 3. Install a skill</a></li><li class="chapter-item expanded "><a href="4-list-installed-skills.html"><strong aria-hidden="true">1.3.4.</strong> 4. List installed skills</a></li><li class="chapter-item expanded "><a href="5-remove-a-skill.html"><strong aria-hidden="true">1.3.5.</strong> 5. Remove a skill</a></li></ol></li><li class="chapter-item expanded "><a href="commands.html"><strong aria-hidden="true">1.4.</strong> Commands</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="sources-—-manage-configured-sources.html"><strong aria-hidden="true">1.4.1.</strong> sources — Manage configured sources</a></li><li class="chapter-item expanded "><a href="platforms-—-list-configured-platforms.html"><strong aria-hidden="true">1.4.2.</strong> platforms — List configured platforms</a></li><li class="chapter-item expanded "><a href="add-—-install-a-skill.html"><strong aria-hidden="true">1.4.3.</strong> add — Install a skill</a></li><li class="chapter-item expanded "><a href="remove-—-remove-a-skill.html"><strong aria-hidden="true">1.4.4.</strong> remove — Remove a skill</a></li><li class="chapter-item expanded "><a href="update-—-update-installed-skills.html"><strong aria-hidden="true">1.4.5.</strong> update — Update installed skills</a></li><li class="chapter-item expanded "><a href="restore-—-restore-skills-from-lock-file.html"><strong aria-hidden="true">1.4.6.</strong> restore — Restore skills from lock file</a></li><li class="chapter-item expanded "><a href="list-—-list-installed-skills.html"><strong aria-hidden="true">1.4.7.</strong> list — List installed skills</a></li><li class="chapter-item expanded "><a href="query-—-query-skills-from-a-source.html"><strong aria-hidden="true">1.4.8.</strong> query — Query skills from a source</a></li><li class="chapter-item expanded "><a href="find-—-interactively-find-and-install-a-skill.html"><strong aria-hidden="true">1.4.9.</strong> find — Interactively find and install a skill</a></li><li class="chapter-item expanded "><a href="rec-—-manage-recommended-skills.html"><strong aria-hidden="true">1.4.10.</strong> rec — Manage recommended skills</a></li><li class="chapter-item expanded "><a href="cache-—-manage-skills-cache.html"><strong aria-hidden="true">1.4.11.</strong> cache — Manage skills cache</a></li><li class="chapter-item expanded "><a href="config-—-manage-configuration.html"><strong aria-hidden="true">1.4.12.</strong> config — Manage configuration</a></li><li class="chapter-item expanded "><a href="new-—-create-a-skill-project.html"><strong aria-hidden="true">1.4.13.</strong> new — Create a skill project</a></li></ol></li><li class="chapter-item expanded "><a href="configuration.html"><strong aria-hidden="true">1.5.</strong> Configuration</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="config-file-location.html"><strong aria-hidden="true">1.5.1.</strong> Config file location</a></li><li class="chapter-item expanded "><a href="configuration-structure.html"><strong aria-hidden="true">1.5.2.</strong> Configuration structure</a></li><li class="chapter-item expanded "><a href="full-example.html"><strong aria-hidden="true">1.5.3.</strong> Full example</a></li><li class="chapter-item expanded "><a href="platforms.html"><strong aria-hidden="true">1.5.4.</strong> Platforms</a></li><li class="chapter-item expanded "><a href="sources.html"><strong aria-hidden="true">1.5.5.</strong> Sources</a></li><li class="chapter-item expanded "><a href="recommended.html"><strong aria-hidden="true">1.5.6.</strong> Recommended</a></li><li class="chapter-item expanded "><a href="cache.html"><strong aria-hidden="true">1.5.7.</strong> Cache</a></li><li class="chapter-item expanded "><a href="registry.html"><strong aria-hidden="true">1.5.8.</strong> Registry</a></li><li class="chapter-item expanded "><a href="--from-parameter-resolution.html"><strong aria-hidden="true">1.5.9.</strong> --from parameter resolution</a></li><li class="chapter-item expanded "><a href="--skill-parameter.html"><strong aria-hidden="true">1.5.10.</strong> --skill parameter</a></li><li class="chapter-item expanded "><a href="--agent-validation.html"><strong aria-hidden="true">1.5.11.</strong> --agent validation</a></li><li class="chapter-item expanded "><a href="url-normalization.html"><strong aria-hidden="true">1.5.12.</strong> URL normalization</a></li></ol></li><li class="chapter-item expanded "><a href="installation-model:-canonical-directory-+-symlinks.html"><strong aria-hidden="true">1.6.</strong> Installation Model: Canonical Directory + Symlinks</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="global-vs-local-paths.html"><strong aria-hidden="true">1.6.1.</strong> Global vs local paths</a></li><li class="chapter-item expanded "><a href="symlink-rules.html"><strong aria-hidden="true">1.6.2.</strong> Symlink rules</a></li><li class="chapter-item expanded "><a href="fallback-mechanism.html"><strong aria-hidden="true">1.6.3.</strong> Fallback mechanism</a></li><li class="chapter-item expanded "><a href="platform-directory-behavior.html"><strong aria-hidden="true">1.6.4.</strong> Platform directory behavior</a></li></ol></li><li class="chapter-item expanded "><a href="lock-file.html"><strong aria-hidden="true">1.7.</strong> Lock File</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="locations.html"><strong aria-hidden="true">1.7.1.</strong> Locations</a></li><li class="chapter-item expanded "><a href="format.html"><strong aria-hidden="true">1.7.2.</strong> Format</a></li><li class="chapter-item expanded "><a href="entry-fields.html"><strong aria-hidden="true">1.7.3.</strong> Entry Fields</a></li><li class="chapter-item expanded "><a href="top-level-fields.html"><strong aria-hidden="true">1.7.4.</strong> Top-level Fields</a></li></ol></li><li class="chapter-item expanded "><a href="migrating-from-xskillyaml.html"><strong aria-hidden="true">1.8.</strong> Migrating from .xskill.yaml</a></li><li class="chapter-item expanded "><a href="editor-integration.html"><strong aria-hidden="true">1.9.</strong> Editor Integration</a></li><li class="chapter-item expanded "><a href="development.html"><strong aria-hidden="true">1.10.</strong> Development</a></li><li><ol class="section"><li class="chapter-item expanded "><a href="building.html"><strong aria-hidden="true">1.10.1.</strong> Building</a></li><li class="chapter-item expanded "><a href="testing.html"><strong aria-hidden="true">1.10.2.</strong> Testing</a></li><li class="chapter-item expanded "><a href="project-structure.html"><strong aria-hidden="true">1.10.3.</strong> Project Structure</a></li></ol></li><li class="chapter-item expanded "><a href="license.html"><strong aria-hidden="true">1.11.</strong> License</a></li></ol></li></ol>';
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
