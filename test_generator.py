import socket
import os
from pathlib import Path
import shutil

START_PORT = 7000
CLIENT_PORT = 8888
NODE_PORT = 9999
NODE_CNT = 3


def get_local_ip():
  s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
  try:
    s.connect(("8.8.8.8", 80))
    return s.getsockname()[0]
  finally:
    s.close()


def gen_query_script(node, port):
  filename = f"{TEST_DIR}/{node}/run_query.sh"
  with open(filename, "w") as f:
    f.writelines([
      "#!/bin/sh\n",
      f"./query_service -p {port}"
    ])

  os.chmod(filename, 0o755)
  print(f"Generated {filename}")


def gen_job_script(node, port, query_port):
  filename = f"{TEST_DIR}/{node}/run_job.sh"
  with open(filename, "w") as f:
    f.writelines([
      "#!/bin/sh\n",
      f"./job_service -p {port} -q {LOCAL_IP}:{query_port}"
    ])

  os.chmod(filename, 0o755)
  print(f"Generated {filename}")


def gen_node_script(node, port, gateway_port):
  filename = f"{TEST_DIR}/{node}/run_node.sh"

  db_port = port
  job_port   = port + 1
  node_port  = port + 2

  with open(filename, "w") as f:
    f.writelines([
        "#!/bin/bash\n",
        "set -x\n",
        "\n",
        "pids=()\n",
        "\n",

        f"./db_service -p {db_port} &\n",
        "pids+=($!)\n",
        "\n",

        "sleep 1\n",
        "\n",

        f"./job_service -p {job_port} -d {LOCAL_IP}:{db_port} &\n",
        "pids+=($!)\n",
        "\n",

        "cleanup() {\n",
        "  kill ${pids[@]} 2>/dev/null\n",
        "}\n",
        "\n",

        "trap cleanup EXIT\n",
        "\n",

        "sleep 2\n",
        "\n",

        f"./node -p {node_port} -j {LOCAL_IP}:{job_port} -g {LOCAL_IP}:{gateway_port}\n",
    ])

  os.chmod(filename, 0o755)
  print(f"Generated {filename}")

def gen_gateway_script(client_port, node_port):
  filename = f"{TEST_DIR}/run_gateway.sh"

  with open(filename, "w") as f:
    f.writelines([
      "#!/bin/sh\n",
      "set +x\n",
      f"./gateway -c {client_port} -n {node_port}\n",
    ])

  os.chmod(filename, 0o755)
  print(f"Generated {filename}")


  """
  if not followers:
    with open(filename, "w") as f:
      f.writelines([
        "#!/bin/sh\n",
        f"./node -p {port} -j {LOCAL_IP}:{job_port}"
      ])
  else:
    followers_str = " ".join([f"{LOCAL_IP}:{x}" for x in followers])
    with open(filename, "w") as f:
      f.writelines([
        "#!/bin/sh\n",
        f"./node -p {port} -j {LOCAL_IP}:{job_port} --leader --followers {followers_str}"
      ])
  """


LOCAL_IP = get_local_ip()
HERE = Path(__file__).resolve().parent
TEST_DIR = f"{HERE}/test_environment"

# Generate the test environment directory
os.makedirs(TEST_DIR, exist_ok=True)

# Copy the gateway
shutil.copy(f"{HERE}/target/debug/gateway", f"{TEST_DIR}/")
print(f"Copied {HERE}/target/debug/gateway -> {TEST_DIR}/")

gen_gateway_script(CLIENT_PORT, NODE_PORT)

# Generate the directory for each nodes
for i in range(NODE_CNT):
  node = f"node_{i+1}"
  os.makedirs(f"{TEST_DIR}/{node}", exist_ok=True)

  # Copy the executables
  shutil.copy(f"{HERE}/target/debug/node", f"{TEST_DIR}/{node}")
  print(f"Copied {HERE}/target/debug/node -> {TEST_DIR}/{node}")

  shutil.copy(f"{HERE}/target/debug/job_service", f"{TEST_DIR}/{node}")
  print(f"Copied {HERE}/target/debug/job_service-> {TEST_DIR}/{node}")

  shutil.copy(f"{HERE}/target/debug/query_service", f"{TEST_DIR}/{node}")
  print(f"Copied {HERE}/target/debug/query_service-> {TEST_DIR}/{node}")

  shutil.copy(f"{HERE}/target/debug/db_service", f"{TEST_DIR}/{node}")
  print(f"Copied {HERE}/target/debug/db_service-> {TEST_DIR}/{node}")
  print("")

  gen_node_script(f"{node}", START_PORT, NODE_PORT)
  START_PORT += 1000
  print("")


