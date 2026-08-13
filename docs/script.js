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
 * Download Resolver (GitHub Releases API)
 */
async function resolveDownload() {
    const REPO = 'Xenonesis/sysmon';
    const API_URL = `https://api.github.com/repos/${REPO}/releases/latest`;
    const RELEASES_PAGE = `https://github.com/${REPO}/releases/latest`;
    const CACHE_KEY = 'sysmon_release_cache_v4';
    const CACHE_TTL = 3600000; // 1 hour
    
    const heroBtn = document.getElementById('downloadNow');
    const sectionBtn = document.getElementById('downloadNowSection');
    const info = document.getElementById('downloadInfo');
    const versionDisplay = document.getElementById('latestVersion');
    const changelogTitle = document.querySelector('#changelog .section-title');
    const buttons = [heroBtn, sectionBtn].filter(Boolean);

    /** Update every version-labelled element on the page */
    function applyVersion(version) {
        if (!version) return;
        if (versionDisplay) versionDisplay.textContent = `v${version}`;
        if (changelogTitle) {
            changelogTitle.textContent = `What's new in v${version}`;
        }
        document.querySelectorAll('[data-version-display]').forEach(el => {
            el.textContent = `v${version}`;
        });
    }
    
    /**
     * Apply direct-download URL, version text and info line
     */
    function applyDownload(data) {
        const { url, name, sizeMB, version } = data;
        buttons.forEach(btn => {
            btn.href = url;
            btn.removeAttribute('target');
            btn.rel = '';
        });

        applyVersion(version);

        const parts = [name];
        if (sizeMB) parts.push(`${sizeMB} MB`);
        if (version) parts.push(`v${version}`);

        if (info) {
            info.textContent = parts.join(' · ') + ' — Ready to download';
        }
    }
    
    /**
     * Fallback to GitHub releases page (version text still updated if known)
     */
    function useFallback(version) {
        buttons.forEach(btn => {
            btn.href = RELEASES_PAGE;
            btn.target = '_blank';
            btn.rel = 'noopener';
        });

        if (info) {
            info.textContent = version
                ? `v${version} — download from GitHub Releases`
                : 'Visit GitHub Releases for the latest version';
        }
    }
    
    /**
     * Check cache for valid data
     */
    function getCachedRelease() {
        try {
            const cached = localStorage.getItem(CACHE_KEY);
            if (!cached) return null;
            
            const { timestamp, data } = JSON.parse(cached);
            if (Date.now() - timestamp > CACHE_TTL) {
                localStorage.removeItem(CACHE_KEY);
                return null;
            }
            
            return data;
        } catch {
            return null;
        }
    }
    
    /**
     * Cache release data
     */
    function cacheRelease(data) {
        try {
            localStorage.setItem(CACHE_KEY, JSON.stringify({
                timestamp: Date.now(),
                data
            }));
        } catch {
            // Storage might be full or disabled
        }
    }
    
    // Check cache first
    const cached = getCachedRelease();
    if (cached) {
        if (cached.url.includes('github.com') && !cached.url.includes('download')) {
             useFallback(cached.version);
        } else {
             applyDownload(cached);
        }
        return;
    }
    
    // Fetch from GitHub API
    try {
        const response = await fetch(API_URL, {
            headers: {
                'Accept': 'application/vnd.github.v3+json'
            }
        });
        
        if (!response.ok) {
            // Handle 404 (no releases yet) gracefully
            if (response.status === 404) {
                console.info('No releases found yet. Using releases page fallback.');
                if (info) {
                    info.textContent = 'No releases yet — check back soon!';
                }
            }
            throw new Error('API request failed');
        }
        
        const release = await response.json();
        const version = release.tag_name?.replace(/^v/, '') || release.tag_name;

        // ── Step 1: Always update version text immediately from the tag ──
        applyVersion(version);
        
        // ── Step 2: Find the best downloadable asset ──
        // Prefer *-setup.exe, then any .exe, then fall back to releases page
        const installerAsset = release.assets?.find(a =>
            a.name?.toLowerCase().includes('setup') &&
            a.name?.toLowerCase().endsWith('.exe')
        );
        const exeAsset = installerAsset || release.assets?.find(a =>
            a.name?.toLowerCase().endsWith('.exe')
        );
        
        if (exeAsset?.browser_download_url) {
            const sizeMB = exeAsset.size ? (exeAsset.size / (1024 * 1024)).toFixed(1) : null;
            
            const data = {
                url: exeAsset.browser_download_url,
                name: exeAsset.name,
                sizeMB,
                version
            };
            
            cacheRelease(data);
            applyDownload(data);
        } else {
            // Release exists but has no binary assets yet — point to its page
            buttons.forEach(btn => {
                btn.href = release.html_url || RELEASES_PAGE;
                btn.target = '_blank';
                btn.rel = 'noopener';
            });
            if (info) {
                info.textContent = version
                    ? `v${version} — download from GitHub Releases`
                    : 'Visit GitHub Releases for the latest version';
            }
            // Cache this so we don't hammer the API on every page view
            cacheRelease({
                url: release.html_url || RELEASES_PAGE,
                name: `SystemMonitor-${version}-setup.exe`,
                sizeMB: null,
                version
            });
        }
    } catch (error) {
        console.warn('Failed to fetch release info:', error);
        useFallback();
    }
}

/**
 * Parallax effect for hero (subtle)
 */
function initParallax() {
    const hero = document.querySelector('.hero-visual');
    if (!hero || window.matchMedia('(prefers-reduced-motion: reduce)').matches) return;
    
    window.addEventListener('scroll', () => {
        const scrollY = window.scrollY;
        const translateY = scrollY * 0.1;
        hero.style.transform = `translateY(${translateY}px)`;
    }, { passive: true });
}

/**
 * Initialize parallax on load
 */
window.addEventListener('load', () => {
    initParallax();
});
