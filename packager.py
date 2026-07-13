import socket
import os
from pathlib import Path
import shutil

CLIENT_PORT = 8888
NODE_PORT = 9999

def get_local_ip():
  s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
  try:
    s.connect(("8.8.8.8", 80))
    return s.getsockname()[0]
  finally:
    s.close()


LOCAL_IP = get_local_ip()
HERE = Path(__file__).resolve().parent
OUT_DIR = f"{HERE}/constella_db"
os.makedirs(OUT_DIR, exist_ok=True)

def copy(src, dest):
  shutil.copy(src, dest)
  print(f"Copied {src} -> {dest}")


if __name__ == "__main__":
  copy(f"{HERE}/target/debug/constella_db", f"{OUT_DIR}/")
  copy(f"{HERE}/target/debug/gateway", f"{OUT_DIR}/")
  copy(f"{HERE}/target/debug/node", f"{OUT_DIR}/")
  copy(f"{HERE}/target/debug/job_service", f"{OUT_DIR}/")
  copy(f"{HERE}/target/debug/db_service", f"{OUT_DIR}/")

  with open(f"{OUT_DIR}/.env", "w") as f:
    f.writelines([
      f"GATEWAY_IP={LOCAL_IP}\n",
      f"GATEWAY_CLIENT_PORT={CLIENT_PORT}\n",
      f"GATEWAY_NODE_PORT={NODE_PORT}\n",
    ])

  print(f"\n'constella_db' has been created at {HERE}.")
