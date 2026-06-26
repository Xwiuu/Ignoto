package main

import (
	"fmt"
	"io"
	"net/http"
	"time"

	"golang.org/x/net/proxy"
)

func main() {
	// 1. Define o túnel SOCKS5 do Tor (Fail-Close absoluto)
	dialer, err := proxy.SOCKS5("tcp", "127.0.0.1:9050", nil, proxy.Direct)
	if err != nil {
		fmt.Printf("❌ ERRO FATAL: Falha ao conectar no Tor. O Docker tá rodando?\nDetalhes: %v\n", err)
		return
	}

	// 2. Configura o Client HTTP para nunca dar bypass no túnel
	httpTransport := &http.Transport{
		Dial: dialer.Dial,
	}
	
	client := &http.Client{
		Transport: httpTransport,
		Timeout:   15 * time.Second, // Tor pode ser um pouco mais lento
	}

	fmt.Println("🕵️ Iniciando Canário Externo: roteando via rede Tor...")

	// 3. O Tiro no serviço de eco (sem vazar DNS local)
	resp, err := client.Get("http://checkip.amazonaws.com")
	if err != nil {
		fmt.Printf("❌ ERRO FATAL: Vazamento bloqueado. Não foi possível sair pela rede Tor.\nDetalhes: %v\n", err)
		return
	}
	defer resp.Body.Close()

	body, _ := io.ReadAll(resp.Body)
	fmt.Printf("✅ SUCESSO! Túnel selado.\n🌐 Seu IP de Saída Público (Tor Node) é: %s", string(body))
}
