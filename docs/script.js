/**
 * SysMon — Minimalist Luxury Website
 * Refined interactions and elegant animations
 */

document.addEventListener('DOMContentLoaded', () => {
    // Initialize all modules
    initTheme();
    initDynamicYear();
    initNavigation();
    initSmoothScroll();
    initScrollReveal();
    initNavScrollEffect();
    resolveDownload();
});

/**
 * Theme Management
 */
function initTheme() {
    const THEME_KEY = 'sysmon_theme';
    const themeToggle = document.getElementById('themeToggle');
    
    // Get saved theme or system preference
    function getPreferredTheme() {
        const saved = localStorage.getItem(THEME_KEY);
        if (saved) return saved;
        
        // Check system preference
        if (window.matchMedia('(prefers-color-scheme: dark)').matches) {
            return 'dark';
        }
        return 'light';
    }
    
    // Apply theme
    function applyTheme(theme) {
        document.documentElement.setAttribute('data-theme', theme);
        localStorage.setItem(THEME_KEY, theme);
        
        // Update toggle button aria-label
        if (themeToggle) {
            themeToggle.setAttribute('aria-label',
                theme === 'dark' ? 'Switch to light mode' : 'Switch to dark mode'
            );
        }
    }
    
    // Toggle theme
    function toggleTheme() {
        const current = document.documentElement.getAttribute('data-theme') || 'light';
        const next = current === 'dark' ? 'light' : 'dark';
        applyTheme(next);
    }
    
    // Initialize
    applyTheme(getPreferredTheme());
    
    // Listen for toggle clicks
    if (themeToggle) {
        themeToggle.addEventListener('click', toggleTheme);
    }
    
    // Listen for system theme changes
    window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', (e) => {
        if (!localStorage.getItem(THEME_KEY)) {
            applyTheme(e.matches ? 'dark' : 'light');
        }
    });
}

/**
 * Dynamic Year
 */
function initDynamicYear() {
    const yearEl = document.getElementById('currentYear');
    if (yearEl) {
        yearEl.textContent = new Date().getFullYear();
    }
}

/**
 * Navigation Toggle (Mobile)
 */
function initNavigation() {
    const toggle = document.getElementById('navToggle');
    const links = document.getElementById('navLinks');
    
    if (!toggle || !links) return;
    
    toggle.addEventListener('click', () => {
        const isOpen = links.classList.toggle('open');
        toggle.classList.toggle('active', isOpen);
        toggle.setAttribute('aria-expanded', String(isOpen));
        
        // Prevent body scroll when menu is open
        document.body.style.overflow = isOpen ? 'hidden' : '';
    });
    
    // Close menu when clicking a link
    links.querySelectorAll('a').forEach(link => {
        link.addEventListener('click', () => {
            links.classList.remove('open');
            toggle.classList.remove('active');
            toggle.setAttribute('aria-expanded', 'false');
            document.body.style.overflow = '';
        });
    });
    
    // Close menu on escape key
    document.addEventListener('keydown', (e) => {
        if (e.key === 'Escape' && links.classList.contains('open')) {
            links.classList.remove('open');
            toggle.classList.remove('active');
            toggle.setAttribute('aria-expanded', 'false');
            document.body.style.overflow = '';
        }
    });
}

/**
 * Smooth Scroll for Anchor Links
 */
function initSmoothScroll() {
    document.querySelectorAll('a[href^="#"]').forEach(link => {
        link.addEventListener('click', (e) => {
            const id = link.getAttribute('href').slice(1);
            const el = document.getElementById(id);
            
            if (el) {
                e.preventDefault();
                
                const navHeight = document.querySelector('.nav')?.offsetHeight || 72;
                const targetPosition = el.getBoundingClientRect().top + window.pageYOffset - navHeight;
                
                window.scrollTo({
                    top: targetPosition,
                    behavior: 'smooth'
                });
            }
        });
    });
}

/**
 * Scroll Reveal Animation
 */
function initScrollReveal() {
    const revealElements = document.querySelectorAll(
        '.feature-card, .stat-item, .doc-card, .section-header, .download-card'
    );
    
    if (!revealElements.length) return;
    
    // Add reveal class to elements
    revealElements.forEach(el => el.classList.add('reveal'));
    
    // Check if IntersectionObserver is supported
    if (!('IntersectionObserver' in window)) {
        revealElements.forEach(el => el.classList.add('visible'));
        return;
    }
    
    const observerOptions = {
        threshold: 0.15,
        rootMargin: '0px 0px -48px 0px'
    };
    
    const observer = new IntersectionObserver((entries) => {
        entries.forEach((entry, index) => {
            if (entry.isIntersecting) {
                // Stagger the animation
                const delay = index * 80;
                setTimeout(() => {
                    entry.target.classList.add('visible');
                }, delay);
                observer.unobserve(entry.target);
            }
        });
    }, observerOptions);
    
    revealElements.forEach(el => observer.observe(el));
}

/**
 * Navigation Scroll Effect
 */
function initNavScrollEffect() {
    const nav = document.querySelector('.nav');
    if (!nav) return;
    
    let ticking = false;
    let lastScrollY = 0;
    
    const updateNav = () => {
        const scrollY = window.scrollY;
        
        // Add/remove scrolled class
        nav.classList.toggle('scrolled', scrollY > 20);
        
        // Hide nav on scroll down, show on scroll up (optional enhancement)
        // Uncomment below for hide-on-scroll behavior
        /*
        if (scrollY > lastScrollY && scrollY > 100) {
            nav.style.transform = 'translateY(-100%)';
        } else {
            nav.style.transform = 'translateY(0)';
        }
        */
        
        lastScrollY = scrollY;
        ticking = false;
    };
    
    window.addEventListener('scroll', () => {
        if (!ticking) {
            requestAnimationFrame(updateNav);
            ticking = true;
        }
    }, { passive: true });
    
    // Initial check
    updateNav();
}

/**
 * Download Resolver — always serves the latest GitHub release.
 *
 * Strategy:
 *  1. Read the cached ETag + release data from localStorage.
 *  2. Hit the API with If-None-Match — GitHub returns 304 (no body) if nothing changed;
 *     costs zero rate-limit quota and resolves in ~50 ms.
 *  3. On 200, parse the new release, update all version displays + download buttons,
 *     and write the fresh ETag back to cache.
 *  4. On any network/API failure, use cached data or fall back to the releases page.
 *
 * Result: the page always reflects a new release on the very next page load, not after
 * a 1-hour TTL window.
 */
async function resolveDownload() {
    const REPO          = 'Xenonesis/sysmon';
    const API_URL       = `https://api.github.com/repos/${REPO}/releases/latest`;
    const RELEASES_PAGE = `https://github.com/${REPO}/releases/latest`;
    const CACHE_KEY     = 'sysmon_dl_v5';   // bump if cache schema changes

    // ── DOM refs ────────────────────────────────────────────────────────────
    const heroBtn      = document.getElementById('downloadNow');
    const sectionBtn   = document.getElementById('downloadNowSection');
    const infoEl       = document.getElementById('downloadInfo');
    const verDisplay   = document.getElementById('latestVersion');
    const clTitle      = document.querySelector('#changelog .section-title');
    const buttons      = [heroBtn, sectionBtn].filter(Boolean);

    // ── Helpers ──────────────────────────────────────────────────────────────

    /** Set every version-labelled element to vX.Y.Z */
    function applyVersion(ver) {
        if (!ver) return;
        const label = `v${ver}`;
        if (verDisplay) verDisplay.textContent = label;
        if (clTitle)    clTitle.textContent    = `What's new in ${label}`;
        document.querySelectorAll('[data-version-display]')
                .forEach(el => { el.textContent = label; });
    }

    /** Wire buttons to a direct download URL */
    function applyDirectDownload(url, name, sizeMB, ver) {
        buttons.forEach(btn => {
            btn.href = url;
            btn.removeAttribute('target');
            btn.removeAttribute('rel');
        });
        applyVersion(ver);
        if (infoEl) {
            const parts = [name];
            if (sizeMB) parts.push(`${sizeMB} MB`);
            if (ver)    parts.push(`v${ver}`);
            infoEl.textContent = parts.join(' · ') + ' — Ready to download';
        }
    }

    /** Wire buttons to the GitHub releases page (when no binary asset exists) */
    function applyReleasePage(pageUrl, ver) {
        buttons.forEach(btn => {
            btn.href   = pageUrl || RELEASES_PAGE;
            btn.target = '_blank';
            btn.rel    = 'noopener';
        });
        applyVersion(ver);
        if (infoEl) {
            infoEl.textContent = ver
                ? `v${ver} — download from GitHub Releases`
                : 'Visit GitHub Releases for the latest version';
        }
    }

    /** Generic error fallback */
    function applyFallback() {
        buttons.forEach(btn => {
            btn.href   = RELEASES_PAGE;
            btn.target = '_blank';
            btn.rel    = 'noopener';
        });
        if (infoEl) infoEl.textContent = 'Visit GitHub Releases for the latest version';
    }

    /** Load cache — returns { etag, data } or null */
    function loadCache() {
        try {
            const raw = localStorage.getItem(CACHE_KEY);
            return raw ? JSON.parse(raw) : null;
        } catch { return null; }
    }

    /** Save { etag, data } to cache */
    function saveCache(etag, data) {
        try {
            localStorage.setItem(CACHE_KEY, JSON.stringify({ etag, data }));
        } catch { /* storage full / disabled */ }
    }

    /** Pick the best downloadable asset from a release's asset list */
    function pickAsset(assets) {
        if (!Array.isArray(assets) || assets.length === 0) return null;
        // Strictly require *-setup.exe installer; never serve portable exe.
        return assets.find(a => a.name?.toLowerCase().includes('setup') && a.name?.toLowerCase().endsWith('.exe'))
            || null;
    }

    /** Convert a GitHub release JSON object → our internal data shape */
    function releaseToData(release) {
        const ver   = release.tag_name?.replace(/^v/, '') ?? release.tag_name;
        const asset = pickAsset(release.assets);
        return {
            version:   ver,
            assetUrl:  asset?.browser_download_url ?? null,
            assetName: asset?.name ?? `SystemMonitor-${ver}-setup.exe`,
            sizeMB:    asset?.size ? (asset.size / (1024 * 1024)).toFixed(1) : null,
            pageUrl:   release.html_url ?? RELEASES_PAGE,
        };
    }

    /** Apply a data object to the page */
    function applyData(data) {
        if (data.assetUrl) {
            applyDirectDownload(data.assetUrl, data.assetName, data.sizeMB, data.version);
        } else {
            applyReleasePage(data.pageUrl, data.version);
        }
    }

    // ── Main flow ────────────────────────────────────────────────────────────

    const cache = loadCache();

    // Immediately render from cache so the page never flashes "Resolving…"
    if (cache?.data) applyData(cache.data);

    // Always revalidate against the API (ETag makes this nearly free when unchanged)
    try {
        const headers = { 'Accept': 'application/vnd.github.v3+json' };
        if (cache?.etag) headers['If-None-Match'] = cache.etag;

        const res = await fetch(API_URL, { headers });

        if (res.status === 304) {
            // GitHub confirmed: nothing changed — cache is authoritative, nothing to do
            return;
        }

        if (!res.ok) {
            if (res.status === 404) {
                if (infoEl) infoEl.textContent = 'No releases yet — check back soon!';
            }
            // Keep whatever we showed from cache; don't call applyFallback() if cache worked
            if (!cache?.data) applyFallback();
            return;
        }

        const release = await res.json();
        const etag    = res.headers.get('ETag') ?? '';
        const data    = releaseToData(release);

        saveCache(etag, data);
        applyData(data);         // update page with freshly confirmed latest

    } catch (err) {
        console.warn('[SysMon] Release check failed:', err);
        if (!cache?.data) applyFallback();
        // else: cached data already rendered — silent degradation
    }
}

/** Parallax effect for hero (subtle) */
function initParallax() {
    const hero = document.querySelector('.hero-visual');
    if (!hero || window.matchMedia('(prefers-reduced-motion: reduce)').matches) return;

    window.addEventListener('scroll', () => {
        hero.style.transform = `translateY(${window.scrollY * 0.1}px)`;
    }, { passive: true });
}

window.addEventListener('load', () => {
    initParallax();
});
