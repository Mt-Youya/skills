#!/usr/bin/env python3
"""fetch-rss.py — 批量抓取RSS并输出配对的 title/link/pubDate
每条记录一行：FEED_URL\tTITLE\tLINK\tPUBDATE（tab分隔）

两阶段分批策略（用户指定规则）：
第一阶段：每批3条并行；失败的批次记录下来并延后
第二阶段：所有批次完成后，把失败的URL重新分组，每批2条继续处理
"""
import sys
import time
import re
import urllib.request
import xml.etree.ElementTree as ET
from concurrent.futures import ThreadPoolExecutor

BATCH_SIZE = 3       # 第一阶段每批条数
RETRY_BATCH_SIZE = 2 # 第二阶段（重试）每批条数
BATCH_SLEEP = 2      # 批间sleep秒数
FETCH_TIMEOUT = 10
# 502退避重试：502且响应体<100字节判定为出网代理限流
RETRY_BACKOFF = 5    # 退避秒数（不能太长，否则总时长超出4分钟bash超时）
MAX_RETRIES = 1      # 最多重试1次

def load_urls():
    """从同目录 fetch-rss.sh 的 shell 数组中提取全部 URL，按出现顺序去重。"""
    import os
    sh_path = os.path.join(os.path.dirname(os.path.abspath(__file__)), "fetch-rss.sh")
    urls = []
    seen = set()
    with open(sh_path) as f:
        for line in f:
            for m in re.finditer(r'"(https?://[^"]+)"', line):
                u = m.group(1)
                if u not in seen:
                    seen.add(u)
                    urls.append(u)
    return urls

def strip_ns(tag):
    return tag.rsplit('}', 1)[-1] if '}' in tag else tag

def fetch_feed(url):
    """抓取单个RSS，返回 (url, items)。502短响应体判定为代理限流，退避重试。"""
    data = None
    for attempt in range(MAX_RETRIES + 1):
        try:
            req = urllib.request.Request(url, headers={'User-Agent': 'Mozilla/5.0'})
            with urllib.request.urlopen(req, timeout=FETCH_TIMEOUT) as resp:
                data = resp.read()
            break
        except urllib.error.HTTPError as e:
            body = b''
            try:
                body = e.read()
            except Exception:
                pass
            # 502/503 且响应体极短 → 出网代理限流
            if e.code in (502, 503) and len(body) < 100 and attempt < MAX_RETRIES:
                print(f"RATE_LIMITED\t{url}\tbacking off {RETRY_BACKOFF}s (attempt {attempt+1}/{MAX_RETRIES})", file=sys.stderr)
                time.sleep(RETRY_BACKOFF)
                continue
            return url, []
        except Exception:
            return url, []
    if data is None:
        return url, []
    try:
        root = ET.fromstring(data)
    except ET.ParseError:
        return url, []
    items = []
    for it in root.iter():
        if strip_ns(it.tag) == 'item' or strip_ns(it.tag) == 'entry':
            title = link = pubdate = ''
            for child in it:
                t = strip_ns(child.tag)
                if t == 'title' and not title:
                    title = (child.text or '').strip()
                elif t == 'link' and not link:
                    link = (child.text or '').strip()
                    if not link:
                        link = child.get('href', '')
                elif t in ('pubDate', 'published', 'updated') and not pubdate:
                    pubdate = (child.text or '').strip()
            if title or link:
                items.append((title, link, pubdate))
    return url, items

def process_batch(pool, batch, batch_label):
    """处理一个批次，输出成功条目，返回失败的URL列表。"""
    failed = []
    results = list(pool.map(fetch_feed, batch))
    for url, items in results:
        if items:
            for title, link, pubdate in items:
                print(f"{url}\t{title}\t{link}\t{pubdate}")
        else:
            failed.append(url)
            print(f"FAIL[{batch_label}]\t{url}\t\t", file=sys.stderr)
    return failed

def main():
    urls = load_urls()
    total_items = 0
    ok_feeds = 0
    fail_feeds = 0

    # ===== 第一阶段：每批3条，失败的延后 =====
    print(f"=== 第一阶段：共 {len(urls)} 个RSS，每批{BATCH_SIZE}个并行，批间sleep {BATCH_SLEEP}秒 ===", file=sys.stderr)
    deferred = []  # 延后处理的URL
    with ThreadPoolExecutor(max_workers=BATCH_SIZE) as pool:
        for i in range(0, len(urls), BATCH_SIZE):
            batch = urls[i:i+BATCH_SIZE]
            failed = process_batch(pool, batch, "phase1")
            deferred.extend(failed)
            if i + BATCH_SIZE < len(urls):
                time.sleep(BATCH_SLEEP)

    # ===== 第二阶段：失败的URL重新分组，每批2条继续 =====
    if deferred:
        print(f"=== 第二阶段：{len(deferred)} 个失败URL重试，每批{RETRY_BATCH_SIZE}个 ===", file=sys.stderr)
        # 重试前先歇一下，给代理恢复时间
        time.sleep(BATCH_SLEEP * 3)
        still_failed = []
        with ThreadPoolExecutor(max_workers=RETRY_BATCH_SIZE) as pool:
            for i in range(0, len(deferred), RETRY_BATCH_SIZE):
                batch = deferred[i:i+RETRY_BATCH_SIZE]
                failed = process_batch(pool, batch, "phase2")
                still_failed.extend(failed)
                if i + RETRY_BATCH_SIZE < len(deferred):
                    time.sleep(BATCH_SLEEP)
        fail_feeds = len(still_failed)
        if still_failed:
            print(f"=== 最终失败（重试后仍不通）：{len(still_failed)} 个 ===", file=sys.stderr)
            for u in still_failed:
                print(f"DEAD\t{u}\t\t", file=sys.stderr)
    else:
        print("=== 第一阶段全部成功，无需重试 ===", file=sys.stderr)

    # 统计产出（重新扫stdout不现实，用stderr粗略统计）
    ok_feeds = len(urls) - fail_feeds
    print(f"=== 完成：{ok_feeds}个源有产出/空，{fail_feeds}个源最终失败 ===", file=sys.stderr)

if __name__ == '__main__':
    main()
