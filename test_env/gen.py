import socket
import os


def get_local_ip():
  s = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
  try:
    s.connect(("8.8.8.8", 80))
    return s.getsockname()[0]
  finally:
    s.close()


def gen_query_script(node, port):
  filename = f"{node}/run_query.sh"
  with open(filename, "w") as f:
    f.writelines([
      "#!/bin/sh\n",
      f"./query_service -p {port}"
    ])

  os.chmod(filename, 0o755)
  print(f"Generated {filename}")


def gen_job_script(node, port, query_port):
  filename = f"{node}/run_job.sh"
  with open(filename, "w") as f:
    f.writelines([
      "#!/bin/sh\n",
      f"./job_service -p {port} -q {LOCAL_IP}:{query_port}"
    ])

  os.chmod(filename, 0o755)
  print(f"Generated {filename}")


def gen_node_script(node, port, job_port, followers=None):
  filename = f"{node}/run_node.sh"

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
START_PORT = 7000

leader = "node1"
followers = ["node2", "node3"]
followers_ip = []

# Generate for followers
for follower in followers:
  gen_query_script(follower, START_PORT)
  gen_job_script(follower, START_PORT + 1, START_PORT)
  gen_node_script(follower, START_PORT + 2, START_PORT + 1)
  followers_ip.append(START_PORT + 2)
  START_PORT += 1000

# Generate for leader
gen_query_script(leader, START_PORT)
gen_job_script(leader, START_PORT + 1, START_PORT)
gen_node_script(leader, START_PORT + 2, START_PORT + 1, followers_ip)
