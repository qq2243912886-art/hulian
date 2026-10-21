#!/usr/bin/env python3
"""PDS 个人版 API 真机探针：验证 分片上传 / 断点续传 / 下载 Range / 目录 / 配额。

用法：python3 scripts/pds_probe.py
凭据默认读取 ~/.aliyun/pds_config.json（aliyun CLI PDS 插件配置），也可用环境变量覆盖：
  PDS_ENDPOINT / PDS_DOMAIN_ID / PDS_ACCESS_TOKEN / PDS_DRIVE_ID
所有测试文件都会写进根目录下的「_互联探针」文件夹，结束后自动清理。
"""
import json
import os
import sys
import time
import urllib.request
import urllib.error

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(ROOT, ".cache", "probe")
os.makedirs(OUT, exist_ok=True)


def load_cfg():
    endpoint = os.environ.get("PDS_ENDPOINT")
    domain = os.environ.get("PDS_DOMAIN_ID")
    token = os.environ.get("PDS_ACCESS_TOKEN")
    drive = os.environ.get("PDS_DRIVE_ID")
    if not token:
        p = os.path.expanduser("~/.aliyun/pds_config.json")
        prof = json.load(open(p))["profiles"][0]
        endpoint = endpoint or prof["endpoint"]
        domain = domain or prof["domain_id"]
        token = prof["access_token"]
    endpoint = endpoint if endpoint.startswith("http") else "https://" + endpoint
    return {"endpoint": endpoint.rstrip("/"), "domain": domain, "token": token, "drive": drive or "2"}


CFG = load_cfg()
H = {
    "Authorization": "Bearer " + CFG["token"],
    "Content-Type": "application/json",
    "User-Agent": "hulian-probe/0.1",
}
DRIVE = CFG["drive"]


def redact(o):
    """把响应里敏感的 url/token 截断，方便落盘对照。"""
    if isinstance(o, dict):
        r = {}
        for k, v in o.items():
            if k in ("upload_url", "url", "thumbnail"):
                r[k] = str(v).split("?")[0] + "?<signed>"
            elif k in ("access_token", "refresh_token"):
                r[k] = "<redacted>"
            else:
                r[k] = redact(v)
        return r
    if isinstance(o, list):
        return [redact(x) for x in o]
    return o


def api(path, body, method="POST", save=None):
    req = urllib.request.Request(
        CFG["endpoint"] + path,
        data=json.dumps(body).encode() if body is not None else None,
        method=method,
        headers=H,
    )
    try:
        with urllib.request.urlopen(req, timeout=60) as r:
            raw = r.read().decode()
            status = r.status
    except urllib.error.HTTPError as e:
        raw = e.read().decode()
        status = e.code
    try:
        data = json.loads(raw)
    except Exception:
        data = {"_raw": raw[:800]}
    if save:
        json.dump(
            {"path": path, "status": status, "body": redact(data)},
            open(os.path.join(OUT, save + ".json"), "w"),
            ensure_ascii=False,
            indent=2,
        )
    return status, data


def put_url(url, data, content_range=None):
    # 注意：上传 URL 的 OSS 签名把 Content-Type 纳入签名（签名为空），
    # 因此 PUT 时绝对不要发送 Content-Type，否则 SignatureDoesNotMatch。
    headers = {}
    if content_range:
        headers["Content-Range"] = content_range
    req = urllib.request.Request(url, data=data, method="PUT", headers=headers)
    try:
        with urllib.request.urlopen(req, timeout=300) as r:
            return r.status, dict(r.headers), b""
    except urllib.error.HTTPError as e:
        return e.code, dict(e.headers), e.read()[:600]


def get_url(url, headers=None):
    req = urllib.request.Request(url, method="GET", headers=headers or {})
    try:
        with urllib.request.urlopen(req, timeout=300) as r:
            return r.status, dict(r.headers), r.read()
    except urllib.error.HTTPError as e:
        return e.code, dict(e.headers), e.read()[:400]


def step(name):
    print(f"\n──── {name} ────")


def main():
    print(f"endpoint={CFG['endpoint']} domain={CFG['domain']} drive={DRIVE}")
    token = CFG["token"]
    print(f"token={token[:3]}***{token[-3:]} (len={len(token)})")

    # 0. 用户 & 空间信息
    step("0. user / drive")
    st, user = api("/v2/user/get", {}, save="user_get")
    print("user_get:", st, user.get("nick_name"), user.get("user_id", "")[:8] + "…")
    st, drive = api("/v2/drive/get", {"drive_id": DRIVE}, save="drive_get")
    if st == 200:
        print(
            "drive_get:",
            st,
            drive.get("drive_name"),
            f"used={drive.get('used_size')} total={drive.get('total_size')}",
        )
    else:
        print("drive_get:", st, drive)
    st, quota = api("/v2/drive/get_quota", {"drive_id": DRIVE}, save="drive_quota")
    print("drive_get_quota:", st, redact(quota) if st != 200 else (quota.get("size"), quota.get("used")))

    # 1. 创建探针目录（同名则 auto_rename）
    step("1. create folder")
    st, folder = api(
        "/v2/file/create",
        {
            "drive_id": DRIVE,
            "parent_file_id": "root",
            "name": "_互联探针",
            "type": "folder",
            "check_name_mode": "auto_rename",
        },
        save="folder_create",
    )
    print("folder_create:", st, {k: folder.get(k) for k in ("file_id", "name", "type", "exist")})
    folder_id = folder.get("file_id")

    # 2. 单分片小文件上传
    step("2. upload small (1 part)")
    data = b"hulian probe small " + str(time.time()).encode()
    st, cr = api(
        "/v2/file/create",
        {
            "drive_id": DRIVE,
            "parent_file_id": folder_id,
            "name": "small.txt",
            "type": "file",
            "size": len(data),
            "part_info_list": [{"part_number": 1}],
            "check_name_mode": "auto_rename",
        },
        save="small_create",
    )
    print("create:", st, {k: cr.get(k) for k in ("file_id", "upload_id", "exist", "rapid_upload")})
    print("parts:", [p.get("part_number") for p in (cr.get("part_info_list") or [])])
    st, hd, err = put_url(cr["part_info_list"][0]["upload_url"], data)
    print("put:", st, {k: v for k, v in hd.items() if k.lower() in ("etag", "content-length")}, err[:200])
    st, comp = api(
        "/v2/file/complete",
        {"drive_id": DRIVE, "file_id": cr["file_id"], "upload_id": cr["upload_id"]},
        save="small_complete",
    )
    print("complete:", st, {k: comp.get(k) for k in ("file_id", "name", "size", "status", "content_hash")})

    # 3. 多分片大文件上传（12MB / 3 parts）
    step("3. upload 12MB (3 parts)")
    size = 12 * 1024 * 1024
    payload = os.urandom(size)
    part_count = 3
    st, cr = api(
        "/v2/file/create",
        {
            "drive_id": DRIVE,
            "parent_file_id": folder_id,
            "name": "big.bin",
            "type": "file",
            "size": size,
            "part_info_list": [{"part_number": i + 1} for i in range(part_count)],
            "check_name_mode": "auto_rename",
        },
        save="big_create",
    )
    print("create:", st, {k: cr.get(k) for k in ("file_id", "upload_id", "exist")})
    parts = cr.get("part_info_list") or []
    print("parts:", [(p.get("part_number"), str(p.get("upload_url", "")).split("?")[0][:60]) for p in parts])
    # 分片大小：除最后一片外均分
    chunk = size // part_count
    etags = []
    t0 = time.time()
    for p in parts:
        n = p["part_number"]
        start = (n - 1) * chunk
        end = size if n == part_count else n * chunk
        cur = payload[start:end]
        st, hd, err = put_url(p["upload_url"], cur)
        etag = {k: v for k, v in hd.items() if k.lower() == "etag"}
        etags.append(etag)
        print(f"  part {n}: {st} bytes={len(cur)} etag={etag}")
        if st >= 300:
            print("  !! part failed", err)
    print(f"upload took {time.time() - t0:.2f}s")
    st, comp = api(
        "/v2/file/complete",
        {"drive_id": DRIVE, "file_id": cr["file_id"], "upload_id": cr["upload_id"]},
        save="big_complete",
    )
    print("complete:", st, {k: comp.get(k) for k in ("file_id", "name", "size", "status")})
    big_id = cr["file_id"]

    # 4. 列目录
    step("4. file list")
    st, lst = api(
        "/v2/file/list",
        {
            "drive_id": DRIVE,
            "parent_file_id": folder_id,
            "limit": 50,
            "order_by": "created_at",
            "order_direction": "DESC",
        },
        save="file_list",
    )
    print("list:", st, [(i.get("name"), i.get("size")) for i in lst.get("items", [])])
    print("next_marker:", repr(lst.get("next_marker")))

    # 5. 断点续传相关接口
    step("5. resume helpers")
    st, uploaded = api(
        "/v2/file/list_uploaded_parts",
        {"drive_id": DRIVE, "file_id": big_id, "upload_id": cr["upload_id"]},
        save="list_uploaded_parts",
    )
    print("list_uploaded_parts:", st, json.dumps(redact(uploaded), ensure_ascii=False)[:400])

    # 6. 下载 + Range
    step("6. download + range")
    st, dl = api(
        "/v2/file/get_download_url",
        {"drive_id": DRIVE, "file_id": big_id, "expire_sec": 900},
        save="get_download_url",
    )
    print("get_download_url:", st, {k: dl.get(k) for k in ("method", "size", "expiration")})
    url = dl.get("url")
    st, hd, body = get_url(url)
    print("GET:", st, "len:", len(body), "match:", body == payload)
    st, hd, part = get_url(url, {"Range": "bytes=0-255"})
    print("Range GET:", st, hd.get("Content-Range"), "len:", len(part), "accept-ranges:", hd.get("Accept-Ranges"))

    # 7. 秒传预检（sha1 + proof_code）
    step("7. rapid upload probe (hash only)")
    import hashlib

    sha1 = hashlib.sha1(payload).hexdigest()
    st, pre = api(
        "/v2/file/create",
        {
            "drive_id": DRIVE,
            "parent_file_id": folder_id,
            "name": "big_rapid.bin",
            "type": "file",
            "size": size,
            "content_hash_name": "sha1",
            "content_hash": sha1,
            "check_name_mode": "auto_rename",
        },
        save="rapid_create",
    )
    print("create with sha1:", st, {k: pre.get(k) for k in ("rapid_upload", "exist", "file_id")}, pre.get("code", ""))

    # 8. 删除清理
    step("8. cleanup")
    st, root = api("/v2/file/list", {"drive_id": DRIVE, "parent_file_id": "root", "limit": 100})
    for it in root.get("items", []):
        if it.get("type") == "folder" and it.get("name", "").startswith("_互联探针"):
            st2, ch = api("/v2/file/list", {"drive_id": DRIVE, "parent_file_id": it["file_id"], "limit": 100})
            for c in ch.get("items", []):
                api("/v2/file/delete", {"drive_id": DRIVE, "file_id": c["file_id"]})
            api("/v2/file/delete", {"drive_id": DRIVE, "file_id": it["file_id"]})
            print("cleanup leftover folder:", it.get("name"))
    for fid in [i for i in (big_id, cr.get("file_id")) if i]:
        st, _ = api("/v2/file/delete", {"drive_id": DRIVE, "file_id": fid})
        print("delete", fid[:8], "->", st)
    st, lst = api("/v2/file/list", {"drive_id": DRIVE, "parent_file_id": folder_id, "limit": 100})
    for it in lst.get("items", []):
        st, _ = api("/v2/file/delete", {"drive_id": DRIVE, "file_id": it["file_id"]})
        print("delete", it.get("name"), "->", st)
    st, _ = api("/v2/file/delete", {"drive_id": DRIVE, "file_id": folder_id})
    print("delete folder ->", st, "(回收站，需手动清空)")


if __name__ == "__main__":
    main()
