import re
import sys
import json


def parse_curl(text: str) -> dict[str, str]:
    text = text.replace("\\\n", " ").replace("\r", "").strip()

    config = {}

    cookie_match = re.search(r"-H\s+'Cookie:\s*([^']+)'", text, re.IGNORECASE)
    if not cookie_match:
        cookie_match = re.search(r'-H\s+"Cookie:\s*([^"]+)"', text, re.IGNORECASE)
    if cookie_match:
        config["cookie"] = cookie_match.group(1).strip()

    auth_match = re.search(r"-H\s+'authorization:\s*([^']+)'", text, re.IGNORECASE)
    if not auth_match:
        auth_match = re.search(r'-H\s+"authorization:\s*([^"]+)"', text, re.IGNORECASE)
    if auth_match:
        config["auth_token"] = auth_match.group(1).strip()

    body_match = re.search(r"-d\s+'([^']+)'", text)
    if not body_match:
        body_match = re.search(r'-d\s+"([^"]+)"', text)
    if body_match:
        try:
            body = json.loads(body_match.group(1))
            if "productId" in body:
                config["product_id"] = body["productId"]
        except json.JSONDecodeError:
            pass

    if "cookie" in config:
        for part in config["cookie"].split(";"):
            part = part.strip()
            if part.startswith("bigmodel_token_production="):
                token_in_cookie = part.split("=", 1)[1]
                try:
                    import base64
                    payload = token_in_cookie.split(".")[1]
                    payload += "=" * (4 - len(payload) % 4)
                    data = json.loads(base64.b64decode(payload))
                    if "customer_id" in data:
                        config["customer_id"] = data["customer_id"]
                except Exception:
                    pass
                break

    if "customer_id" not in config and "auth_token" in config:
        try:
            import base64
            payload = config["auth_token"].split(".")[1]
            payload += "=" * (4 - len(payload) % 4)
            data = json.loads(base64.b64decode(payload))
            if "customer_id" in data:
                config["customer_id"] = data["customer_id"]
        except Exception:
            pass

    config.setdefault("product_id", "product-02434c")
    config.setdefault("pay_type", "ALI")
    config.setdefault("rps", 30)
    config.setdefault("timeout_secs", 5)

    return config


def main():
    print("paste curl command, press Enter to confirm:")
    lines = []
    has_content = False
    while True:
        try:
            line = input()
        except EOFError:
            break
        if line.strip() == "":
            if has_content:
                break
        else:
            has_content = True
            lines.append(line)

    raw = "\n".join(line.replace("\r", "") for line in lines)
    if not raw.strip():
        print("no input", file=sys.stderr)
        sys.exit(1)

    config = parse_curl(raw)

    if "auth_token" not in config:
        print("warning: authorization header not found", file=sys.stderr)
    if "cookie" not in config:
        print("warning: Cookie header not found", file=sys.stderr)
    if "customer_id" not in config:
        print("warning: customer_id not found in token", file=sys.stderr)

    with open("config.json", "w", encoding="utf-8") as f:
        json.dump(config, f, indent=2, ensure_ascii=False)

    print(f"\nconfig.json written ({len(config)} keys):")
    for k, v in config.items():
        v_str = str(v)
        display = v_str if len(v_str) <= 60 else v_str[:30] + "..." + v_str[-20:]
        print(f"  {k}: {display}")


if __name__ == "__main__":
    main()
