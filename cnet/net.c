#include "net.h"
#include <stdlib.h>
#include <errno.h>
TcpListener * tcp_listen_port(const char * addr, size_t max_cons){
	int32_t s = socket(AF_INET, SOCK_STREAM,0);
	int32_t opt = 1;
	if (setsockopt(s,SOL_SOCKET,SO_REUSEADDR, (char*)&opt, sizeof(opt))<0){
		return 0;	
	}
        struct sockaddr_in serverAddr;
        serverAddr.sin_family=AF_INET;
        serverAddr.sin_port=htons(PORT);
        inet_pton(AF_INET, addr, &serverAddr.sin_addr);
 	if (bind(s, (struct sockaddr*)&serverAddr, sizeof(serverAddr))<0){
		return 0;
	}
	if(fcntl(s,F_SETFL, O_NONBLOCK) < 0){
		return 0;
	}
	if(listen(s, max_cons)<0){
		return 0;
	}	
	return 0;
}
TcpStreamResult tcp_try_accept(TcpListener * listener){
	TcpStreamResult out;
	out.stream = 0;
	int32_t new_socket = 0;
	int32_t sz = sizeof(listener->addr);
        if((new_socket = accept(listener->sock, (struct sockaddr *)&listener->addr, (socklen_t *)&sz))>= 0){ 
		out.stream = malloc(sizeof(TcpStream));
		out.stream->sock = new_socket;
		out.er = AcceptSuccess;
	}
        // Handle non-blocking accept errors
        if (new_socket < 0 && errno != EAGAIN && errno != EWOULDBLOCK) {
		out.er = AcceptErrorWouldBlock;
		return out;
        }
	out.er = AcceptErrorConFailed;
}
TcpReadError tcp_try_read(TcpStream * reader, unsigned char * bytes, size_t len);

bool tcp_try_write(TcpStream * stream, unsigned char * bytes, size_t len);


