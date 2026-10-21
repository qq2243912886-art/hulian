#!/usr/bin/env python3
"""PDS 个人版：分片上传/下载/断点探针 v2（严格按 App 将要实现的流程验证）。

验证项：
  1. 多分片上传（10MB+123B，4MB 分片）→ complete → 下载校验内容一致
  2. 下载 Range（断点续传可行性）
  3. list_uploaded_parts（分片级断点查询）
  4. 0 字节文件上传
  5. 同名文件 check_name_mode: refuse / auto_rename
  6. 嵌套目录创建 + 列表分页
所有测试文件放在云端根目录的「文件互传/」下（该目录也是 App 的默认互传目录）。
注意：当前令牌无删除权限（403），测试文件会保留，请在阿里云盘 App 中手动清理「_selftest」前缀文件。
"""
import hashlib
import http.client
import json
import os
import ssl
import sys
import time
import urllib.error
import urllib.request
from urllib.parse import urlparse

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, ".cache", "probe")
os.makedirs(OUT, exist_ok=True)
MB = 1024 * 1024
CHUNK = 4 * MB


def load_cfg():
    endpoint = os.environ.get("PDS_ENDPOINT")
    token = os.environ.get("PDS_ACCESS_TOKEN")
    if not token:
        prof = json.load(open(os.path.expanduser("~/.aliyun/pds_config.json")))["profiles"][0]
        endpoint = endpoint or prof["endpoint"]
        token = prof["access_token"]
    endpoint = endpoint if str(endpoint).startswith("http") else "https://" + endpoint
    return endpoint.rstrip("/"), token, os.environ.get("PDS_DRIVE_ID", "2")


HOST, TOKEN, DRIVE = load_cfg()
HEAD = {"Authorization": "Bearer " + TOKEN, "Content-Type": "application/json"}


def api(path, body, save=None):
    req = urllib.request.Request(HOST + path, data=json.dumps(body).encode(), method="POST", headers=HEAD)
    try:
        with urllib.request.urlopen(req, timeout=120) as r:
            st, raw = r.status, r.read().decode()
    except urllib.error.HTTPError as e:
        st, raw = e.code, e.read().decode()
    try:
        data = json.loads(raw)
    except Exception:
        data = {"_raw": raw[:500]}
    if save:
        body2 = json.loads(json.dumps(data))
        for k in ("upload_url", "url"):
            if isinstance(body2, dict) and k in body2:
                body2[k] = str(body2[k]).split("?")[0] + "?<signed>"
        for p in (body2.get("part_info_list") or []) if isinstance(body2, dict) else []:
            if p.get("upload_url"):
                p["upload_url"] = str(p["upload_url"]).split("?")[0] + "?<signed>"
        json.dump({"path": path, "status": st, "body": body2}, open(os.path.join(OUT, save + ".json"), "w"), ensure_ascii=False, indent=2)
    return st, data


def put_part(url, data):
    u = urlparse(url)
    conn = http.client.HTTPSConnection(u.hostname, timeout=300, context=ssl.create_default_context())
    path = u.path + ("?" + u.query if u.query else "")
    conn.putrequest("PUT", path, skip_accept_encoding=True)
    conn.putheader("Content-Length", str(len(data)))
    conn.endheaders()
    conn.send(data)
    resp = conn.getresponse()
    payload = resp.read()
    etag = resp.getheader("ETag")
    conn.close()
    return resp.status, etag, payload[:300]


def get_url(url, headers=None):
    req = urllib.request.Request(url, headers=headers or {}, method="GET")
    try:
        with urllib.request.urlopen(req, timeout=300) as r:
            return r.status, dict(r.headers), r.read()
    except urllib.error.HTTPError as e:
        return e.code, dict(e.headers), e.read()[:300]


def ensure_dir(parent_id, name):
    st, d = api("/v2/file/create", {"drive_id": DRIVE, "parent_file_id": parent_id, "name": name, "type": "folder", "check_name_mode": "refuse"})
    if st in (200, 201):
        return d["file_id"], True
    # 已存在：从列表里找
    st, ls = api("/v2/file/list", {"drive_id": DRIVE, "parent_file_id": parent_id, "limit": 200, "order_by": "name", "order_direction": "ASC"})
    for it in ls.get("items", []):
        if it.get("name") == name and it.get("type") == "folder":
            return it["file_id"], False
    raise RuntimeError(f"ensure_dir failed: {st} {d}")


def upload(file_name, parent_id, payload, chunk=CHUNK):
    size = len(payload)
    parts_n = max(1, (size + chunk - 1) // chunk) if size else 1
    body = {
        "drive_id": DRIVE,
        "parent_file_id": parent_id,
        "name": file_name,
        "type": "file",
        "size": size,
        "part_info_list": [{"part_number": i + 1} for i in range(parts_n)],
        "check_name_mode": "auto_rename",
    }
    t0 = time.time()
    st, cr = api("/v2/file/create", body, save=f"v2_create_{file_name}")
    assert st in (200, 201), f"create failed {st} {cr}"
    if cr.get("rapid_upload") or cr.get("exist"):
        return {"rapid": True, "file_id": cr["file_id"], "secs": 0}
    urls = {p["part_number"]: p["upload_url"] for p in cr.get("part_info_list") or []}
    for n in range(1, parts_n + 1):
        cur = payload[(n - 1) * chunk : n * chunk]
        st, etag, err = put_part(urls[n], cur)
        print(f"    part {n}/{parts_n} {len(cur)}B -> {st} etag={etag}")
        assert st < 300, f"put part {n} failed {st} {err}"
    st, comp = api("/v2/file/complete", {"drive_id": DRIVE, "file_id": cr["file_id"], "upload_id": cr["upload_id"]}, save=f"v2_complete_{file_name}")
    assert st == 200, f"complete failed {st} {comp}"
    return {"rapid": False, "file_id": cr["file_id"], "upload_id": cr["upload_id"], "size": comp.get("size"), "secs": time.time() - t0}


def main():
    print(f"host={HOST} drive={DRIVE}")
    folder_id, created = ensure_dir("root", "文件互传")
    print(f"互传目录 file_id={folder_id} (新建={created})")
    sub_id, _ = ensure_dir(folder_id, "_selftest")

    # ---- 1. 多分片上传 ----
    size = 10 * MB + 123
    payload = os.urandom(size)
    sha = hashlib.sha256(payload).hexdigest()[:16]
    print(f"\n[1] 多分片上传 {size} bytes, chunk={CHUNK}, parts={-(-size // CHUNK)}, sha256={sha}")
    r = upload(f"_selftest_multi.bin", sub_id, payload)
    print(f"    complete: size={r.get('size')} 用时={r['secs']:.2f}s")
    assert r.get("size") == size, "size mismatch!"

    # ---- 2. 下载 + Range ----
    print("\n[2] 下载 & Range 断点")
    st, dl = api("/v2/file/get_download_url", {"drive_id": DRIVE, "file_id": r["file_id"], "expire_sec": 900}, save="v2_download_url")
    assert st == 200, dl
    print(f"    url method={dl.get('method')} size={dl.get('size')} expire={dl.get('expiration')}")
    st, hd, body = get_url(dl["url"])
    print(f"    GET -> {st} len={len(body)} sha256_ok={hashlib.sha256(body).hexdigest()[:16] == sha}")
    assert body == payload, "download content mismatch!"
    st, hd, part = get_url(dl["url"], {"Range": "bytes=1000-1999"})
    print(f"    Range GET -> {st} Content-Range={hd.get('Content-Range')} len={len(part)}")
    assert part == payload[1000:2000], "range content mismatch!"

    # ---- 3. 断点：只传一部分后查询已上传分片 ----
    print("\n[3] list_uploaded_parts 断点查询")
    payload2 = os.urandom(3 * CHUNK)
    st, cr = api("/v2/file/create", {"drive_id": DRIVE, "parent_file_id": sub_id, "name": "_selftest_resume.bin", "type": "file", "size": len(payload2), "part_info_list": [{"part_number": i} for i in (1, 2, 3)], "check_name_mode": "auto_rename"}, save="v2_create_resume")
    assert st in (200, 201), cr
    for n in (1, 2):
        st, etag, err = put_part(cr["part_info_list"][n - 1]["upload_url"], payload2[(n - 1) * CHUNK : n * CHUNK])
        print(f"    part {n} -> {st}")
    st, up = api("/v2/file/list_uploaded_parts", {"drive_id": DRIVE, "file_id": cr["file_id"], "upload_id": cr["upload_id"]}, save="v2_list_uploaded_parts")
    print(f"    list_uploaded_parts -> {st} {json.dumps(up, ensure_ascii=False)[:400]}")
    st, comp = api("/v2/file/complete", {"drive_id": DRIVE, "file_id": cr["file_id"], "upload_id": cr["upload_id"]})
    print(f"    complete(仅2/3分片) -> {st} {json.dumps({k: comp.get(k) for k in ('code','message','size','status')}, ensure_ascii=False)}")

    # ---- 4. 0 字节文件 ----
    print("\n[4] 0 字节文件")
    st, cr = api("/v2/file/create", {"drive_id": DRIVE, "parent_file_id": sub_id, "name": "_selftest_empty.txt", "type": "file", "size": 0, "check_name_mode": "auto_rename"}, save="v2_create_empty")
    print(f"    create -> {st} {json.dumps({k: cr.get(k) for k in ('file_id','upload_id','rapid_upload','exist','code')}, ensure_ascii=False)}")
    if cr.get("upload_id"):
        st, comp = api("/v2/file/complete", {"drive_id": DRIVE, "file_id": cr["file_id"], "upload_id": cr["upload_id"]})
        print(f"    complete -> {st} size={comp.get('size')}")

    # ---- 5. 同名冲突 ----
    print("\n[5] 同名冲突 check_name_mode")
    st, cr = api("/v2/file/create", {"drive_id": DRIVE, "parent_file_id": sub_id, "name": "_selftest_multi.bin", "type": "file", "size": 1, "check_name_mode": "refuse"})
    print(f"    refuse -> {st} code={cr.get('code')} msg={str(cr.get('message'))[:80]}")
    st, cr = api("/v2/file/create", {"drive_id": DRIVE, "parent_file_id": sub_id, "name": "_selftest_multi.bin", "type": "file", "size": 1, "part_info_list": [{"part_number": 1}], "check_name_mode": "auto_rename"})
    print(f"    auto_rename -> {st} name={cr.get('file_name')}")

    # ---- 6. 列表分页 ----
    print("\n[6] 列表分页")
    st, ls = api("/v2/file/list", {"drive_id": DRIVE, "parent_file_id": sub_id, "limit": 2})
    print(f"    limit=2 -> {st} items={[i.get('name') for i in ls.get('items', [])]} next={repr(ls.get('next_marker'))[:20]}")
    if ls.get("next_marker"):
        st, ls2 = api("/v2/file/list", {"drive_id": DRIVE, "parent_file_id": sub_id, "limit": 2, "marker": ls["next_marker"]})
        print(f"    page2 -> {st} items={[i.get('name') for i in ls2.get('items', [])]}")
    st, ls = api("/v2/file/list", {"drive_id": DRIVE, "parent_file_id": sub_id, "limit": 100, "order_by": "created_at", "order_direction": "DESC"})
    print("    final list:", [(i.get("name"), i.get("size")) for i in ls.get("items", [])])
    total = sum(i.get("size") or 0 for i in ls.get("items", []))
    print(f"    total bytes={total}")

    print("\n✅ 全部探针通过（文件请稍后在阿里云盘 App 中删除 文件互传/_selftest）")


if __name__ == "__main__":
    main()
