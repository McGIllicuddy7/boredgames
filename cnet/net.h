#pragma once
#include <arpa/inet.h>
#include <sys/socket.h>
#include <stdbool.h>
#include <stdint.h>
#include <fcntl.h>
#define PORT 8080
typedef struct {
	int32_t sock;
}TcpStream;

typedef struct {
	int32_t sock;
	struct sockaddr_in addr;
}TcpListener;


typedef enum{
	AcceptErrorConFailed, AcceptErrorWouldBlock,AcceptSuccess,
}AcceptError;

typedef enum{
	ReadWouldBlock,
	ReadConnectionClosed,
	ReadSuccess,
} TcpReadError;


typedef struct{
	TcpStream * stream;
	AcceptError er;
} TcpStreamResult;

TcpListener * tcp_listen_port(const char * addr, size_t max_cons, bool * connection_failed);
TcpStreamResult tcp_try_accept(TcpListener * listener);

TcpReadError tcp_try_read(TcpStream * reader, unsigned char * bytes, size_t len);
bool tcp_try_write(TcpStream * stream, unsigned char * bytes, size_t len);




