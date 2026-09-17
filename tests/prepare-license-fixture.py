"""Run the supplied, unmodified PrismKey API with disposable test secrets/data.

No production credential is read. All generated files stay in test-data/prismkey.
Use the Python interpreter from the supplied PrismKey .venv.
"""
import argparse
import json
import secrets
import sys
from datetime import timedelta
from pathlib import Path

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("action", choices=["prepare", "serve", "disable", "enable", "unbind", "expire", "shorten", "outage", "online"])
parser.add_argument("--source", type=Path, required=True, help="Supplied PrismKey project, not a production deployment")
parser.add_argument("--port", type=int, default=18100)
args = parser.parse_args()
root = Path(__file__).resolve().parents[1] / "test-data" / "prismkey"
root.mkdir(parents=True, exist_ok=True)
sys.path.insert(0, str(args.source.resolve() / "backend"))
from app.config import Settings
from app.db import database
from app.models import Base, License, Product, Activation, now
from app.security import Security, make_card

config = Settings(
    _env_file=None, environment="test", database_url="sqlite:///" + (root / "server.db").as_posix(),
    public_origin=f"http://127.0.0.1:{args.port}", cookie_secure=False,
    card_pepper_file=root / "test-pepper.txt", signing_key_file=root / "test-private.pem",
    product_id="NovelWords", issuer="PrismKey_NovelWords", card_prefix="CY", default_product="novel-words",
    # Native checks exercise many requests in one minute; separate unit/UI tests cover 429.
    license_ip_limit=1000, license_card_limit=1000,
)
fixture_path = root / "fixture.json"
if args.action == "prepare":
    if fixture_path.exists():
        raise SystemExit("Fixture already exists; use a fresh dedicated test directory")
    from cryptography.hazmat.primitives import serialization
    from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey
    key = Ed25519PrivateKey.generate()
    config.signing_key_file.write_bytes(key.private_bytes(serialization.Encoding.PEM, serialization.PrivateFormat.PKCS8, serialization.NoEncryption()))
    (root / "test-public.pem").write_bytes(key.public_key().public_bytes(serialization.Encoding.PEM, serialization.PublicFormat.SubjectPublicKeyInfo))
    config.card_pepper_file.write_text(secrets.token_hex(32), encoding="ascii")
    security = Security(config)
    engine, sessions = database(config)
    Base.metadata.create_all(engine)
    fixture = {}
    with sessions.begin() as db:
        products = [Product(slug="novel-words", name="词阅", product_id="NovelWords", issuer="PrismKey_NovelWords", card_prefix="CY"),
                    Product(slug="default", name="Other product", product_id="XHS_Download", issuer="XHS_License_Server", card_prefix="XHS")]
        db.add_all(products)
        db.flush()
        for alias, product, plan in [("primary", products[0], "30d"), ("lifetime", products[0], "lifetime"), ("other", products[1], "30d")]:
            card = make_card(product.card_prefix)
            row = License(product_id=product.id, digest=security.card_digest(card), tail=card[-8:], plan=plan, note="Disposable native integration fixture")
            db.add(row)
            db.flush()
            fixture[alias] = {"cardKey": card, "licenseId": row.id}
    fixture_path.write_text(json.dumps(fixture), encoding="utf-8")
    engine.dispose()
    print("Prepared isolated PrismKey fixture and public key; secrets were not printed")
elif args.action == "serve":
    from app.api import create_app
    from fastapi.responses import JSONResponse
    import uvicorn
    application = create_app(config)
    @application.middleware("http")
    async def simulated_outage(request, call_next):
        if (root / "outage.flag").exists():
            return JSONResponse({"error": {"code": "TEST_OUTAGE"}}, status_code=503)
        return await call_next(request)
    uvicorn.run(application, host="127.0.0.1", port=args.port, access_log=False, log_level="warning")
elif args.action in ("outage", "online"):
    flag = root / "outage.flag"
    if args.action == "outage":
        flag.write_text("test only", encoding="ascii")
    else:
        flag.unlink(missing_ok=True)
    print("Isolated service availability updated")
else:
    fixture = json.loads(fixture_path.read_text(encoding="utf-8"))
    engine, sessions = database(config)
    with sessions.begin() as db:
        card = db.get(License, fixture["primary"]["licenseId"])
        if args.action in ("disable", "enable"):
            card.disabled = args.action == "disable"
            card.grant_version += 1
        elif args.action == "unbind":
            card.device_hash = None
            card.grant_version += 1
            from sqlalchemy import select
            for binding in db.scalars(select(Activation).where(Activation.license_id == card.id, Activation.unbound_at.is_(None))):
                binding.unbound_at = now()
        elif args.action == "expire":
            card.expires_at = now() - timedelta(seconds=1)
            card.grant_version += 1
        elif args.action == "shorten":
            card.expires_at = now() + timedelta(seconds=4)
            # Model an old 30-day grant near its deadline, keeping server invariants.
            card.activated_at = card.expires_at - timedelta(days=30)
            card.grant_version += 1
    engine.dispose()
    print("Isolated license state updated")
