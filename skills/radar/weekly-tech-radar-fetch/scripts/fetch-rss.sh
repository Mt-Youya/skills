#!/bin/bash
# fetch-rss.sh — 批量抓取RSS并提取 title/link/pubDate
# 用法: bash scripts/fetch-rss.sh
# 依赖: curl, grep (支持 -P 正则)
# 输出: 每个RSS源的条目，格式为 "源URL | 标题 | 链接 | 发布日期"

set -euo pipefail

# Layer 1 官方RSS (10个)
LAYER1_OFFICIAL=(
  "https://openai.com/news/rss.xml"
  "https://blog.cloudflare.com/rss/"
  "https://earendil.com/posts/feed.rss"
  "https://deepmind.google/blog/rss.xml"
  "https://huggingface.co/blog/feed.xml"
  "https://github.blog/feed/"
  "https://simonwillison.net/atom/everything/"
  "https://thinkingmachines.ai/blog/index.xml"
  "https://vllm.ai/blog/rss.xml"
  "https://lucumr.pocoo.org/feed.xml"
)

# Layer 1 第三方抓取feed (5个)
LAYER1_FEEDS=(
  "https://raw.githubusercontent.com/Olshansk/rss-feeds/main/feeds/feed_openai_engineering.xml"
  "https://raw.githubusercontent.com/Olshansk/rss-feeds/main/feeds/feed_anthropic_news.xml"
  "https://raw.githubusercontent.com/Olshansk/rss-feeds/main/feeds/feed_anthropic_engineering.xml"
  "https://raw.githubusercontent.com/Olshansk/rss-feeds/main/feeds/feed_claude.xml"
  "https://raw.githubusercontent.com/leontloveless/ai-rss-feeds/main/feeds/cursor-blog.xml"
)

# Layer 1 Google/大厂博客 (13个)
LAYER1_BIGTECH=(
  "https://blog.google/technology/ai/rss/"
  "https://research.google/blog/rss/"
  "https://netflixtechblog.com/feed"
  "https://aws.amazon.com/blogs/aws/feed/"
  "https://vercel.com/atom"
  "https://supabase.com/rss.xml"
  "https://stripe.com/blog/feed.rss"
  "https://engineering.fb.com/feed/"
  "https://github.blog/changelog/feed/"
  "https://github.blog/changelog/label/copilot/feed/"
  "https://developers.googleblog.com/feeds/posts/default/"
  "https://hacks.mozilla.org/feed/"
  "https://engineering.atspotify.com/feed/"
)

# Layer 1 AI工具Releases (5个)
LAYER1_RELEASES=(
  "https://github.com/openai/codex/releases.atom"
  "https://github.com/anthropics/claude-code/releases.atom"
  "https://github.com/google-gemini/gemini-cli/releases.atom"
  "https://github.com/modelcontextprotocol/specification/releases.atom"
  "https://github.com/modelcontextprotocol/servers/releases.atom"
)

# Layer 1 AI专题 (8个)
LAYER1_AI=(
  "https://rss.arxiv.org/rss/cs.AI"
  "https://rss.arxiv.org/rss/cs.LG"
  "https://rss.arxiv.org/rss/cs.CL"
  "https://hnrss.org/newest?q=AI"
  "https://hnrss.org/newest?q=LLM"
  "https://hnrss.org/newest?q=OpenClaw"
  "https://github.com/openclaw/openclaw/releases.atom"
  "https://github.com/openclaw/openclaw/commits/main.atom"
)

# Layer 1 安全 (7个)
LAYER1_SECURITY=(
  "https://krebsonsecurity.com/feed/"
  "https://feeds.feedburner.com/TheHackersNews"
  "https://www.schneier.com/feed/"
  "https://www.cisa.gov/news.xml"
  "https://security.googleblog.com/atom.xml"
  "https://www.freebuf.com/feed"
  "https://api.anquanke.com/data/v1/rss"
)

# Layer 1 前端设计 (9个)
LAYER1_FRONTEND=(
  "https://www.smashingmagazine.com/feed/"
  "https://alistapart.com/main/feed/"
  "https://tympanus.net/codrops/feed/"
  "https://css-tricks.com/feed/"
  "https://astro.build/rss.xml"
  "https://svelte.dev/blog/rss.xml"
  "https://nuxt.com/blog/rss.xml"
  "https://tailwindcss.com/feeds/feed.xml"
  "https://dribbble.com/stories.rss"
)

# Layer 1 编程语言 (8个)
LAYER1_LANGS=(
  "https://blog.vuejs.org/feed.rss"
  "https://blog.rust-lang.org/feed.xml"
  "https://go.dev/blog/feed.atom"
  "https://blog.python.org/feeds/posts/default"
  "https://nodejs.org/en/feed/blog.xml"
  "https://devblogs.microsoft.com/typescript/feed/"
  "https://www.swift.org/atom.xml"
  "https://blog.jetbrains.com/kotlin/feed/"
)

# Layer 1 技术周刊 (4个)
LAYER1_WEEKLY=(
  "https://javascriptweekly.com/rss/"
  "https://this-week-in-rust.org/atom.xml"
  "https://golangweekly.com/rss/"
  "https://blog.bytebytego.com/feed"
)

# Layer 1 开发工具 (5个)
LAYER1_DEVTOLS=(
  "https://github.com/astral-sh/uv/releases.atom"
  "https://github.com/zed-industries/zed/releases.atom"
  "https://github.com/oven-sh/bun/releases.atom"
  "https://github.com/biomejs/biome/releases.atom"
  "https://github.com/langchain-ai/langchain/releases.atom"
)

# Layer 1 学术论文 (2个)
LAYER1_ACADEMIC=(
  "https://www.nature.com/nature.rss"
  "https://rss.arxiv.org/rss/cs.CV"
)

# Layer 3 科技媒体 (29个)
LAYER3_MEDIA=(
  "https://www.technologyreview.com/feed/"
  "http://feeds.arstechnica.com/arstechnica/index"
  "https://www.wired.com/feed/rss"
  "https://www.theverge.com/rss/index.xml"
  "https://techcrunch.com/feed/"
  "https://venturebeat.com/category/ai/feed/"
  "https://36kr.com/feed"
  "https://rsshub.app/huxiu/article"
  "https://rsshub.app/geekpark/news"
  "https://rsshub.app/qbitai/news"
  "https://rsshub.app/jiqizhixin/articles"
  "https://www.ithome.com/rss/"
  "https://rsshub.app/tmtpost/news"
  "https://sspai.com/feed"
  "https://www.ruanyifeng.com/blog/atom.xml"
  "https://hnrss.org/frontpage"
  "https://hnrss.org/best"
  "https://hnrss.org/newest"
  "https://hnrss.org/ask"
  "https://hnrss.org/show"
  "https://hnrss.org/newest?q=AI"
  "https://hnrss.org/newest?q=LLM"
  "https://linux.do/latest.rss"
  "https://www.v2ex.com/index.xml"
  "https://www.v2ex.com/feed/tab/tech.xml"
  "https://rss.nodeseek.com/"
  "https://react.dev/rss.xml"
  "https://nextjs.org/feed.xml"
  "https://developer.chrome.com/blog/feed.xml"
  "https://dev.to/feed"
  "https://www.producthunt.com/feed"
)

# 提取函数：从RSS XML中提取 title/link/pubDate
extract_items() {
  local url="$1"
  local xml
  xml=$(curl -sL --max-time 10 "$url" 2>/dev/null) || return 0
  
  if [ -z "$xml" ]; then
    echo "TIMEOUT|${url}"
    return 0
  fi
  
  # 提取 title, link, pubDate
  # RSS 2.0: <item><title>...</title><link>...</link><pubDate>...</pubDate></item>
  # Atom: <entry><title>...</title><link href="..."/><published>...</published></entry>
  
  if echo "$xml" | grep -q '<entry'; then
    # Atom format
    echo "$xml" | grep -oP '(?<=<title type="text">)[^<]+' | while read -r title; do
      echo "ITEM|${url}|${title}|"
    done
    echo "$xml" | grep -oP '(?<=<link href=")[^"]+' | while read -r link; do
      echo "LINK|${url}|${link}"
    done
  else
    # RSS 2.0 format
    echo "$xml" | grep -oP '(?<=<link>)[^<]+' | while read -r link; do
      echo "LINK|${url}|${link}"
    done
    echo "$xml" | grep -oP '(?<=<title>)[^<]+' | while read -r title; do
      echo "ITEM|${url}|${title}|"
    done
  fi
}

# 批量并行抓取函数：每批5个URL并行，批间sleep 2秒
# 用法: batch_fetch "url1" "url2" ... 或 batch_fetch "${array[@]}"
batch_fetch() {
  local urls=("$@")
  local batch_size=5
  local total=${#urls[@]}
  local i=0
  
  while [ $i -lt $total ]; do
    # 取当前批次的URL（最多batch_size个）
    local batch=()
    local j=0
    while [ $j -lt $batch_size ] && [ $((i+j)) -lt $total ]; do
      batch+=("${urls[$((i+j))]}")
      j=$((j+1))
    done
    
    # 并行发起当前批次
    for url in "${batch[@]}"; do
      extract_items "$url" &
    done
    wait
    
    i=$((i+batch_size))
    # 批间sleep 2秒，避免触发代理限流
    if [ $i -lt $total ]; then
      sleep 2
    fi
  done
}

# 主函数
echo "=== Layer 1 RSS 抓取开始 ==="
ALL_LAYER1_URLS=()
for group_name in LAYER1_OFFICIAL LAYER1_FEEDS LAYER1_BIGTECH LAYER1_RELEASES LAYER1_AI LAYER1_SECURITY LAYER1_FRONTEND LAYER1_LANGS LAYER1_WEEKLY LAYER1_DEVTOLS LAYER1_ACADEMIC; do
  eval "urls=(\"\${${group_name}[@]}\")"
  echo "--- ${group_name} (${#urls[@]}个) ---"
  ALL_LAYER1_URLS+=("${urls[@]}")
done
echo "Layer 1 总计 ${#ALL_LAYER1_URLS[@]} 个RSS，每批5个并行抓取"
batch_fetch "${ALL_LAYER1_URLS[@]}"

echo ""
echo "=== Layer 3 RSS 抓取开始 ==="
echo "Layer 3 总计 ${#LAYER3_MEDIA[@]} 个RSS，每批5个并行抓取"
batch_fetch "${LAYER3_MEDIA[@]}"

echo ""
echo "=== 抓取结束 ==="
