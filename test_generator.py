import socket
import os
from pathlib import Path
import shutil

START_PORT = 7000
LEADER = "leader"
FOLLOWERS = ["follower_1", "follower_2"]


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


def gen_node_script(node, port, job_port, followers=None):
  filename = f"{TEST_DIR}/{node}/run_node.sh"

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

  os.chmod(filename, 0o755)
  print(f"Generated {filename}")


LOCAL_IP = get_local_ip()
HERE = Path(__file__).resolve().parent
TEST_DIR = f"{HERE}/test_environment"

all_nodes = [LEADER, *FOLLOWERS]
followers_ip = []

# Generate the test environment directory
os.makedirs(TEST_DIR, exist_ok=True)

# Generate the directory for each nodes
for node in all_nodes:
  os.makedirs(f"{TEST_DIR}/{node}", exist_ok=True)

  # Copy the executables
  shutil.copy(f"{HERE}/target/debug/node", f"{TEST_DIR}/{node}")
  print(f"Copied {HERE}/target/debug/node -> {TEST_DIR}/{node}")

  shutil.copy(f"{HERE}/target/debug/job_service", f"{TEST_DIR}/{node}")
  print(f"Copied {HERE}/target/debug/job_service-> {TEST_DIR}/{node}")

  shutil.copy(f"{HERE}/target/debug/query_service", f"{TEST_DIR}/{node}")
  print(f"Copied {HERE}/target/debug/query_service-> {TEST_DIR}/{node}")
  print("")

# Generate for followers
for follower in FOLLOWERS:
  gen_query_script(follower, START_PORT)
  gen_job_script(follower, START_PORT + 1, START_PORT)
  gen_node_script(follower, START_PORT + 2, START_PORT + 1)
  followers_ip.append(START_PORT + 2)
  print("")
  START_PORT += 1000

# Generate for leader
gen_query_script(LEADER, START_PORT)
gen_job_script(LEADER, START_PORT + 1, START_PORT)
gen_node_script(LEADER, START_PORT + 2, START_PORT + 1, followers_ip)
