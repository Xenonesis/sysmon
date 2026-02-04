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

    // Download button click tracking (optional)
    const downloadButtons = document.querySelectorAll('.btn-download, .btn-primary[href*="releases"]');

    downloadButtons.forEach(button => {
        button.addEventListener('click', function() {
            // You could add analytics tracking here
            console.log('Download initiated');
        });
    });

    // Add loading animation to download buttons
    downloadButtons.forEach(button => {
        button.addEventListener('click', function() {
            const originalText = this.innerHTML;
            this.innerHTML = '<span class="download-icon">⏳</span> Downloading...';
            this.disabled = true;

            // Reset after 3 seconds (simulating download start)
            setTimeout(() => {
                this.innerHTML = originalText;
                this.disabled = false;
            }, 3000);
        });
    });

    // Add current year to footer
    const yearElement = document.querySelector('.footer-bottom p');
    if (yearElement) {
        const currentYear = new Date().getFullYear();
        yearElement.innerHTML = yearElement.innerHTML.replace('2024', currentYear);
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

    // Try to resolve a direct download - prioritize local downloads folder
    async function findDirectDownload() {
        const localCandidates = [
            'downloads/SystemMonitor-latest.zip',
            'downloads/SystemMonitor-v1.0.0.zip',
            'downloads/system-monitor-latest.exe',
            'downloads/system-monitor-1.0.0.exe'
        ];
        
        const githubCandidates = [
            'SystemMonitor-v1.0.0.zip',
            'system-monitor-setup.exe',
            'system-monitor-installer.exe',
            'system-monitor.exe',
            'SystemMonitor.exe'
        ];
        
        const base = 'https://github.com/Xenonesis/sysmon/releases/latest/download/';
        const releases = 'https://github.com/Xenonesis/sysmon/releases/latest';

        const heroBtn = document.getElementById('downloadNow');
        const sectionBtn = document.getElementById('downloadNowSection');
        const info = document.getElementById('downloadInfo');
        const buttons = [heroBtn, sectionBtn].filter(Boolean);

        // Try local downloads folder first
        for (const localPath of localCandidates) {
            try {
                const resp = await fetch(localPath, { method: 'HEAD' });
                if (resp && resp.ok) {
                    const fileName = localPath.split('/').pop();
                    buttons.forEach(b => {
                        b.href = localPath;
                        b.setAttribute('download', fileName);
                        b.target = '_blank';
                    });
                    if (info) info.textContent = `${fileName} • Latest version available locally`;
                    console.log('Local download found:', localPath);
                    return;
                }
            } catch (err) {
                console.log('Local file not found:', localPath);
            }
        }

        // Try GitHub releases as fallback
        for (const name of githubCandidates) {
            const url = base + name;
            try {
                const resp = await fetch(url, { method: 'HEAD' });
                if (resp && resp.ok) {
                    buttons.forEach(b => {
                        b.href = url;
                        b.setAttribute('download', name);
                        b.target = '_blank';
                    });
                    if (info) info.textContent = `${name} • Downloading from GitHub`;
                    console.log('GitHub download found:', url);
                    return;
                }
            } catch (err) {
                console.log('GitHub HEAD failed for', url, err);
            }
        }

        // Final fallback: point to Releases page
        buttons.forEach(b => {
            b.href = releases;
            b.target = '_blank';
        });
        if (info) info.textContent = 'Visit Releases page for latest version';
        console.log('No direct download found; pointing to Releases page.');
    }

    findDirectDownload();
    
    // Check for updates periodically (every 5 minutes)
    setInterval(findDirectDownload, 300000);
});