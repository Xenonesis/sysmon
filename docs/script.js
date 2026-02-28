// SysMon — Terminal Noir website JS
document.addEventListener('DOMContentLoaded', () => {
    // ---- Dynamic year ----
    const yearEl = document.getElementById('currentYear');
    if (yearEl) yearEl.textContent = new Date().getFullYear();

    // ---- Mobile nav toggle ----
    const toggle = document.getElementById('navToggle');
    const links = document.getElementById('navLinks');
    if (toggle && links) {
        toggle.addEventListener('click', () => {
            const open = links.classList.toggle('open');
            toggle.classList.toggle('active', open);
            toggle.setAttribute('aria-expanded', String(open));
        });
        links.querySelectorAll('a').forEach(a =>
            a.addEventListener('click', () => {
                links.classList.remove('open');
                toggle.classList.remove('active');
                toggle.setAttribute('aria-expanded', 'false');
            })
        );
    }

    // ---- Smooth scroll for anchor links ----
    document.querySelectorAll('a[href^="#"]').forEach(link => {
        link.addEventListener('click', e => {
            const id = link.getAttribute('href').slice(1);
            const el = document.getElementById(id);
            if (el) {
                e.preventDefault();
                el.scrollIntoView({ behavior: 'smooth', block: 'start' });
            }
        });
    });

    // ---- Nav scroll effect ----
    const nav = document.querySelector('nav');
    if (nav) {
        let ticking = false;
        window.addEventListener('scroll', () => {
            if (!ticking) {
                requestAnimationFrame(() => {
                    nav.classList.toggle('scrolled', window.scrollY > 40);
                    ticking = false;
                });
                ticking = true;
            }
        }, { passive: true });
    }

    // ---- Staggered reveal on scroll ----
    const revealTargets = document.querySelectorAll('.feature-card, .doc-link, .ribbon-item');
    if (revealTargets.length && 'IntersectionObserver' in window) {
        const io = new IntersectionObserver((entries) => {
            entries.forEach(entry => {
                if (entry.isIntersecting) {
                    entry.target.classList.add('visible');
                    io.unobserve(entry.target);
                }
            });
        }, { threshold: 0.15, rootMargin: '0px 0px -40px 0px' });

        revealTargets.forEach((el, i) => {
            el.style.transitionDelay = `${(i % 6) * 80}ms`;
            io.observe(el);
        });
    } else {
        revealTargets.forEach(el => el.classList.add('visible'));
    }

    // ---- Download resolver (GitHub Releases API, CORS-safe) ----
    resolveDownload();
});

async function resolveDownload() {
    const REPO = 'Xenonesis/sysmon';
    const API_URL = `https://api.github.com/repos/${REPO}/releases/latest`;
    const RELEASES_PAGE = `https://github.com/${REPO}/releases/latest`;
    const CACHE_KEY = 'sysmon_release_cache';
    const CACHE_TTL = 3600000; // 1 hour

    const heroBtn = document.getElementById('downloadNow');
    const sectionBtn = document.getElementById('downloadNowSection');
    const info = document.getElementById('downloadInfo');
    const buttons = [heroBtn, sectionBtn].filter(Boolean);

    function apply(url, name, sizeMB, ver) {
        buttons.forEach(b => { b.href = url; b.removeAttribute('target'); });
        const parts = [name];
        if (sizeMB) parts.push(`${sizeMB} MB`);
        if (ver) parts.push(ver);
        if (info) info.textContent = parts.join(' · ') + ' — Ready';
    }

    function fallback() {
        buttons.forEach(b => { b.href = RELEASES_PAGE; b.target = '_blank'; });
        if (info) info.textContent = 'Visit GitHub Releases for latest version';
    }

    // 1. Cache check
    try {
        const c = JSON.parse(sessionStorage.getItem(CACHE_KEY));
        if (c && Date.now() - c.ts < CACHE_TTL) { apply(c.url, c.name, c.size, c.ver); return; }
    } catch (_) { /* miss */ }

    // 2. Local downloads folder (GitHub Pages)
    for (const f of ['downloads/SystemMonitor-latest.exe', 'downloads/system-monitor-latest.exe']) {
        try {
            const r = await fetch(f, { method: 'HEAD' });
            if (r.ok) {
                const bytes = parseInt(r.headers.get('content-length') || '0');
                if (bytes < 102400) continue; // skip stubs
                apply(f, f.split('/').pop(), (bytes / 1048576).toFixed(2), null);
                return;
            }
        } catch (_) { /* next */ }
    }

    // 3. GitHub Releases API
    try {
        const r = await fetch(API_URL);
        if (!r.ok) throw new Error(r.status);
        const rel = await r.json();
        const ver = rel.tag_name || '';
        const asset = (rel.assets || []).find(a => a.name.endsWith('.exe'))
                   || (rel.assets || []).find(a => a.name.endsWith('.zip'));
        if (asset) {
            const mb = asset.size ? (asset.size / 1048576).toFixed(2) : null;
            try { sessionStorage.setItem(CACHE_KEY, JSON.stringify({ url: asset.browser_download_url, name: asset.name, size: mb, ver, ts: Date.now() })); } catch (_) {}
            apply(asset.browser_download_url, asset.name, mb, ver);
            return;
        }
    } catch (_) { /* fall through */ }

    // 4. Fallback
    fallback();
}