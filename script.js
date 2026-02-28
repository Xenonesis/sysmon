// System Monitor Website JavaScript

document.addEventListener('DOMContentLoaded', function() {
    // Smooth scrolling for navigation links
    const navLinks = document.querySelectorAll('nav a[href^="#"]');

    navLinks.forEach(link => {
        link.addEventListener('click', function(e) {
            e.preventDefault();

            const targetId = this.getAttribute('href').substring(1);
            const targetElement = document.getElementById(targetId);

            if (targetElement) {
                const offsetTop = targetElement.offsetTop - 70; // Account for fixed nav
                window.scrollTo({
                    top: offsetTop,
                    behavior: 'smooth'
                });
            }
        });
    });

    // Add animation to feature cards on scroll
    const observerOptions = {
        threshold: 0.1,
        rootMargin: '0px 0px -50px 0px'
    };

    const observer = new IntersectionObserver(function(entries) {
        entries.forEach(entry => {
            if (entry.isIntersecting) {
                entry.target.style.opacity = '1';
                entry.target.style.transform = 'translateY(0)';
            }
        });
    }, observerOptions);

    // Observe feature cards
    const featureCards = document.querySelectorAll('.feature-card');
    featureCards.forEach(card => {
        card.style.opacity = '0';
        card.style.transform = 'translateY(20px)';
        card.style.transition = 'opacity 0.6s ease, transform 0.6s ease';
        observer.observe(card);
    });

    // Observe doc cards
    const docCards = document.querySelectorAll('.doc-card');
    docCards.forEach(card => {
        card.style.opacity = '0';
        card.style.transform = 'translateY(20px)';
        card.style.transition = 'opacity 0.6s ease, transform 0.6s ease';
        observer.observe(card);
    });

    // Set dynamic year in footer
    const yearSpan = document.getElementById('currentYear');
    if (yearSpan) {
        yearSpan.textContent = new Date().getFullYear();
    }

    // Add scroll effect to navigation
    let lastScrollTop = 0;
    const navbar = document.querySelector('nav');

    window.addEventListener('scroll', function() {
        const scrollTop = window.pageYOffset || document.documentElement.scrollTop;

        if (scrollTop > lastScrollTop && scrollTop > 100) {
            // Scrolling down
            navbar.style.transform = 'translateY(-100%)';
        } else {
            // Scrolling up
            navbar.style.transform = 'translateY(0)';
        }

        lastScrollTop = scrollTop <= 0 ? 0 : scrollTop;

        // Add background blur on scroll
        if (scrollTop > 50) {
            navbar.style.backdropFilter = 'blur(10px)';
            navbar.style.background = 'rgba(255, 255, 255, 0.95)';
        } else {
            navbar.style.backdropFilter = 'none';
            navbar.style.background = 'var(--background-color)';
        }
    });

    // Add keyboard navigation support
    document.addEventListener('keydown', function(e) {
        // ESC key to close any open modals (if added later)
        if (e.key === 'Escape') {
            // Close modal logic here
        }
    });

    // Add intersection observer for hero section animations
    const heroObserver = new IntersectionObserver(function(entries) {
        entries.forEach(entry => {
            if (entry.isIntersecting) {
                entry.target.style.opacity = '1';
                entry.target.style.transform = 'translateY(0)';
            }
        });
    }, { threshold: 0.1 });

    const heroContent = document.querySelector('.hero-content');
    if (heroContent) {
        heroContent.style.opacity = '0';
        heroContent.style.transform = 'translateY(30px)';
        heroContent.style.transition = 'opacity 0.8s ease, transform 0.8s ease';
        heroObserver.observe(heroContent);
    }

    const heroImage = document.querySelector('.hero-image');
    if (heroImage) {
        heroImage.style.opacity = '0';
        heroImage.style.transform = 'translateY(30px)';
        heroImage.style.transition = 'opacity 0.8s ease 0.2s, transform 0.8s ease 0.2s';
        heroObserver.observe(heroImage);
    }

    // Resolve real download URL via GitHub Releases API (CORS-safe)
    async function findDirectDownload() {
        const REPO = 'Xenonesis/sysmon';
        const API_URL = `https://api.github.com/repos/${REPO}/releases/latest`;
        const RELEASES_PAGE = `https://github.com/${REPO}/releases/latest`;
        const CACHE_KEY = 'sysmon_release_cache';
        const CACHE_TTL = 3600000; // 1 hour in ms

        const heroBtn = document.getElementById('downloadNow');
        const sectionBtn = document.getElementById('downloadNowSection');
        const info = document.getElementById('downloadInfo');
        const buttons = [heroBtn, sectionBtn].filter(Boolean);

        function applyDownload(url, fileName, sizeMB, version) {
            buttons.forEach(b => {
                b.href = url;
                b.removeAttribute('target');
            });
            const sizeStr = sizeMB ? ` (${sizeMB} MB)` : '';
            const versionStr = version ? ` • ${version}` : '';
            if (info) info.textContent = `${fileName}${sizeStr}${versionStr} • Ready to download`;
        }

        function fallbackToReleasesPage() {
            buttons.forEach(b => {
                b.href = RELEASES_PAGE;
                b.target = '_blank';
            });
            if (info) info.textContent = 'Visit GitHub Releases for latest version';
        }

        // Check sessionStorage cache first to avoid GitHub API rate limits (60/hr)
        try {
            const cached = JSON.parse(sessionStorage.getItem(CACHE_KEY));
            if (cached && (Date.now() - cached.timestamp) < CACHE_TTL) {
                applyDownload(cached.url, cached.fileName, cached.sizeMB, cached.version);
                console.log('Download resolved from cache:', cached.fileName);
                return;
            }
        } catch (e) { /* cache miss or parse error, continue */ }

        // Try local downloads folder first (works when hosted on GitHub Pages)
        const localCandidates = [
            'downloads/SystemMonitor-latest.exe',
            'downloads/system-monitor-latest.exe'
        ];

        for (const localPath of localCandidates) {
            try {
                const resp = await fetch(localPath, { method: 'HEAD' });
                if (resp && resp.ok) {
                    const fileSize = resp.headers.get('content-length');
                    const sizeBytes = fileSize ? parseInt(fileSize) : 0;
                    // Skip placeholder/stub files (real binary is > 100KB)
                    if (sizeBytes < 102400) continue;
                    const fileName = localPath.split('/').pop();
                    const sizeMB = (sizeBytes / 1048576).toFixed(2);
                    applyDownload(localPath, fileName, sizeMB, null);
                    console.log('Local download found:', localPath);
                    return;
                }
            } catch (e) { /* not found, continue */ }
        }

        // Query GitHub Releases API (supports CORS, unlike direct asset HEAD requests)
        try {
            const resp = await fetch(API_URL);
            if (!resp.ok) throw new Error(`API returned ${resp.status}`);
            const release = await resp.json();
            const version = release.tag_name || '';
            const assets = release.assets || [];

            // Prefer .exe over .zip
            const exeAsset = assets.find(a => a.name.endsWith('.exe'));
            const zipAsset = assets.find(a => a.name.endsWith('.zip'));
            const asset = exeAsset || zipAsset;

            if (asset) {
                const sizeMB = asset.size ? (asset.size / 1048576).toFixed(2) : null;
                const downloadUrl = asset.browser_download_url;

                // Cache the result
                try {
                    sessionStorage.setItem(CACHE_KEY, JSON.stringify({
                        url: downloadUrl,
                        fileName: asset.name,
                        sizeMB: sizeMB,
                        version: version,
                        timestamp: Date.now()
                    }));
                } catch (e) { /* storage full or unavailable */ }

                applyDownload(downloadUrl, asset.name, sizeMB, version);
                console.log('GitHub API download resolved:', asset.name, version);
                return;
            }
        } catch (err) {
            console.warn('GitHub API request failed:', err.message);
        }

        // Final fallback
        fallbackToReleasesPage();
        console.log('No direct download found; pointing to Releases page.');
    }

    findDirectDownload();
});