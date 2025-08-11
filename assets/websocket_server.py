#!/usr/bin/env python3
import asyncio
import websockets


async def handle_client(websocket):
    print(f"New client connected from {websocket.remote_address}")

    # Send hello message when client connects
    await websocket.send("hello")

    try:
        async for message in websocket:
            print(f"Received: {message}")
            # Echo back with prefix
            response = f"echo:{message}"
            await websocket.send(response)
            print(f"Sent: {response}")
    except websockets.exceptions.ConnectionClosed:
        print(f"Client {websocket.remote_address} disconnected")


async def main():
    # Start WebSocket server on localhost:8765
    server = await websockets.serve(handle_client, "0.0.0.0", 8765)
    print("WebSocket server started on ws://localhost:8765")

    # Keep server running
    await server.wait_closed()


if __name__ == "__main__":
    asyncio.run(main())
