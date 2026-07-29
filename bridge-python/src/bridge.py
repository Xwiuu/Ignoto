import sys

from substrateinterface import SubstrateInterface
import requests


class TorExitClient:
    def __init__(self, proxy_host="127.0.0.1", proxy_port=9050):
        self.proxies = {
            "http": f"socks5://{proxy_host}:{proxy_port}",
            "https": f"socks5://{proxy_host}:{proxy_port}",
        }

    def send_payment(self, url, payload):
        try:
            resp = requests.post(url, json=payload, proxies=self.proxies, timeout=30)
            if resp.status_code == 200:
                print("[+] Payment relayed successfully.")
            else:
                print(f"[-] API returned status {resp.status_code}.")
        except requests.exceptions.ConnectionError:
            print("[!] Proxy connection failed (fail-close). Aborting.")
            sys.exit(1)
        except Exception as e:
            print(f"[!] Unexpected error during exit: {e}")
            sys.exit(1)


class SubstrateListener:
    PALLET = "Ignoto"
    EVENTS = ("ShieldedWithdraw", "Transferred")
    CHAIN_URL = "ws://127.0.0.1:9944"

    def __init__(self, exit_client, api_url):
        self.exit_client = exit_client
        self.api_url = api_url
        self.substrate = None

    def connect(self):
        try:
            self.substrate = SubstrateInterface(url=self.CHAIN_URL)
            print("[+] Connected to Ignoto node.")
        except Exception as e:
            print(f"[!] Failed to connect to node: {e}")
            sys.exit(1)

    def _process_event(self, event):
        try:
            mid = event.value.get("module_id")
            eid = event.value.get("event_id")
            attrs = event.value.get("attributes", {})

            if mid == self.PALLET and eid == "ShieldedWithdraw":
                amount = attrs.get("amount")
                external_address = attrs.get("external_address")
                if amount is not None and external_address is not None:
                    print("[+] ShieldedWithdraw detected.")
                    self.exit_client.send_payment(self.api_url, {
                        "amount": amount,
                        "address": external_address,
                    })

            elif mid == self.PALLET and eid == "Transferred":
                print("[+] Transferred event detected.")
                self.exit_client.send_payment(self.api_url, {
                    "amount": 0,
                    "address": "transferred-event-test",
                })

        except Exception as e:
            print(f"[!] Error processing event: {e}")

    def _block_handler(self, obj, update_nr, subscription_id):
        try:
            h = obj.get("header", {})
            block_num = h.get("number")
            if block_num is None:
                return
            block_hash = self.substrate.get_block_hash(block_num)
            if not block_hash:
                return
            events = self.substrate.get_events(block_hash)
            for event in events:
                self._process_event(event)
        except Exception as e:
            print(f"[!] Error processing block: {e}")

    def listen(self):
        self.connect()
        print("[*] Listening for ShieldedWithdraw events...")
        try:
            self.substrate.subscribe_block_headers(self._block_handler)
        except Exception as e:
            print(f"[!] Listener error: {e}")
            sys.exit(1)


class ExitBridge:
    def __init__(self, api_url="http://majesticbank.sc/api/v1/pay"):
        self.exit_client = TorExitClient()
        self.listener = SubstrateListener(self.exit_client, api_url)

    def run(self):
        self.listener.listen()
