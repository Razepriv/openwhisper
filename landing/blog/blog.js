/* ═══════════════════════════════════════════════════════════════════════════
 * The OpenVoice Dispatch — manifest renderer
 *
 * Reads /blog/manifest.json at runtime, filters posts by `publish <= today`
 * AND `draft === false`, then paints the listing. Category chips filter
 * the list in-memory (no server round-trip). The "Coming next" block
 * surfaces up-to-3 scheduled-but-not-yet-live posts so visitors see the
 * cadence and can subscribe.
 *
 * Why this lives client-side and not in a build step:
 *   1) The 30-day drip can flip a post live without a Vercel rebuild —
 *      a GitHub Actions cron edits manifest.json and triggers a redeploy.
 *      But once the manifest is correct, render is pure browser-side
 *      filtering, which keeps the static site genuinely static.
 *   2) Filter chips are cheap when the data lives in memory.
 *
 * Date math: We compare `YYYY-MM-DD` strings lexicographically rather
 * than building Date objects, because the publish date is calendar-day
 * authored (no timezone implied) and string compare gives the right
 * answer everywhere. Today is taken as the visitor's local YYYY-MM-DD.
 * ═══════════════════════════════════════════════════════════════════════ */

(function () {
  "use strict";

  // Visitor-local YYYY-MM-DD. We don't try to pin to UTC — a post that
  // says "publish: 2026-06-04" should be visible to anyone for whom it's
  // already June 4th, even if it's still June 3rd in UTC.
  function todayLocal() {
    const d = new Date();
    const y = d.getFullYear();
    const m = String(d.getMonth() + 1).padStart(2, "0");
    const day = String(d.getDate()).padStart(2, "0");
    return y + "-" + m + "-" + day;
  }

  // Format a YYYY-MM-DD string as "Jun 4" for the post card meta block.
  function formatShortDate(isoDate) {
    const [y, m, d] = isoDate.split("-").map(Number);
    const monthNames = [
      "Jan",
      "Feb",
      "Mar",
      "Apr",
      "May",
      "Jun",
      "Jul",
      "Aug",
      "Sep",
      "Oct",
      "Nov",
      "Dec",
    ];
    return `${monthNames[m - 1]} ${d}`;
  }

  function escapeHtml(s) {
    if (s == null) return "";
    return String(s)
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;")
      .replace(/'/g, "&#39;");
  }

  /**
   * Render a single post card as HTML. The card links to
   * /blog/posts/<slug>.html — pages we author by hand or stub via
   * the build script, so each post has its own URL for SEO.
   */
  function renderPostCard(post, categoryLabel) {
    const dayLabel = post.day === 0 ? "Day 0" : `Day ${post.day}`;
    return `
      <a class="post-card" href="/blog/posts/${escapeHtml(post.slug)}.html">
        <div class="post-card-meta">
          <span class="post-card-day">${escapeHtml(dayLabel)}</span>
          <span>${escapeHtml(formatShortDate(post.publish))}</span>
          <span>${post.readMinutes} min</span>
        </div>
        <div class="post-card-body">
          <span class="post-card-kicker">${escapeHtml(categoryLabel)}</span>
          <h2 class="post-card-title">${escapeHtml(post.title)}</h2>
          <p class="post-card-dek">${escapeHtml(post.dek)}</p>
        </div>
        <span class="post-card-arrow" aria-hidden="true">→</span>
      </a>
    `;
  }

  /**
   * Render a single upcoming-strip row — a smaller, link-less teaser
   * for posts that aren't live yet. We intentionally don't link these:
   * the page doesn't exist yet, and we don't want crawlers to index
   * 404s during the drip period.
   */
  function renderUpcomingItem(post, categoryLabel) {
    return `
      <li class="upcoming-item">
        <span class="upcoming-when">${escapeHtml(formatShortDate(post.publish))} · ${escapeHtml(categoryLabel)}</span>
        <span class="upcoming-headline"><strong>${escapeHtml(post.title)}</strong> — ${escapeHtml(post.dek)}</span>
      </li>
    `;
  }

  /**
   * Main render — takes the full manifest, filters by category, and
   * paints the post list and (always) the upcoming strip. Called once
   * after fetch, and again every time the user clicks a filter chip.
   */
  function render(manifest, activeCategory) {
    const today = todayLocal();
    const allPosts = manifest.posts || [];
    const categories = manifest.categories || {};

    // Live posts: publish date has arrived AND the maintainers haven't
    // marked it draft. We sort newest-first so the latest post is at
    // the top of the list — that's what visitors are most likely
    // looking for if they're returning.
    let livePosts = allPosts.filter(
      (p) => !p.draft && p.publish <= today
    );

    // Category filter applies after the publish gate so the "All" count
    // is the true live-post count.
    const liveTotal = livePosts.length;
    if (activeCategory && activeCategory !== "all") {
      livePosts = livePosts.filter((p) => p.category === activeCategory);
    }

    livePosts.sort((a, b) =>
      a.publish < b.publish ? 1 : a.publish > b.publish ? -1 : 0
    );

    // Paint the list.
    const listEl = document.getElementById("postList");
    const emptyEl = document.getElementById("postListEmpty");
    if (livePosts.length === 0) {
      listEl.innerHTML = "";
      emptyEl.hidden = false;
    } else {
      emptyEl.hidden = true;
      listEl.innerHTML = livePosts
        .map((p) => {
          const cat = categories[p.category] || {};
          return renderPostCard(p, cat.label || p.category);
        })
        .join("");
    }

    // Update the live count chip under the filter.
    const countEl = document.getElementById("postCount");
    if (countEl) {
      const filteredCount = livePosts.length;
      if (activeCategory === "all" || !activeCategory) {
        countEl.textContent = `${liveTotal} ${liveTotal === 1 ? "post" : "posts"} live · ${30 - liveTotal} scheduled`;
      } else {
        countEl.textContent = `${filteredCount} of ${liveTotal} live ${liveTotal === 1 ? "post" : "posts"} in this category`;
      }
    }

    // Upcoming strip — first 3 not-yet-published, regardless of filter.
    // Showing this even when the user has filtered a category keeps the
    // "this is a daily drip" narrative front-and-center.
    const upcoming = allPosts
      .filter((p) => p.publish > today)
      .sort((a, b) => (a.publish < b.publish ? -1 : 1))
      .slice(0, 3);

    const upcomingSection = document.getElementById("upcomingSection");
    const upcomingList = document.getElementById("upcomingList");
    if (upcoming.length === 0) {
      upcomingSection.hidden = true;
    } else {
      upcomingSection.hidden = false;
      upcomingList.innerHTML = upcoming
        .map((p) => {
          const cat = categories[p.category] || {};
          return renderUpcomingItem(p, cat.label || p.category);
        })
        .join("");
    }
  }

  /**
   * Wire up category chip clicks. Toggles the chip-active class and
   * re-renders the list.
   */
  function wireFilter(manifest) {
    const filterEl = document.getElementById("categoryFilter");
    if (!filterEl) return;
    filterEl.addEventListener("click", (ev) => {
      const btn = ev.target.closest(".chip");
      if (!btn) return;
      const category = btn.getAttribute("data-category");
      if (!category) return;
      // Update chip state
      filterEl
        .querySelectorAll(".chip")
        .forEach((c) => {
          c.classList.remove("chip-active");
          c.setAttribute("aria-selected", "false");
        });
      btn.classList.add("chip-active");
      btn.setAttribute("aria-selected", "true");
      render(manifest, category);
    });
  }

  // Boot — fetch the manifest, render once, wire up the chips.
  async function boot() {
    try {
      const res = await fetch("/blog/manifest.json", { cache: "no-store" });
      if (!res.ok) throw new Error(`manifest fetch failed: ${res.status}`);
      const manifest = await res.json();
      render(manifest, "all");
      wireFilter(manifest);
    } catch (err) {
      // eslint-disable-next-line no-console
      console.error("[OpenVoice blog] Failed to load manifest", err);
      const listEl = document.getElementById("postList");
      if (listEl) {
        listEl.innerHTML = `
          <p class="post-list-empty">
            Couldn't load the post manifest. Try refreshing, or check
            <a href="/blog/feed.xml">the RSS feed</a> directly.
          </p>
        `;
      }
    }
  }

  // Defer until DOM is ready. script tags use `defer`, but belt-and-braces.
  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", boot);
  } else {
    boot();
  }
})();
