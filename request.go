package main

import (
	"crypto/tls"
	"crypto/x509"
	"fmt"
	"io"
	"net/http"
	"os"
)

func main() {
	// Load the CA certificate
	caCert, err := os.ReadFile("cert.pem")
	if err != nil {
		fmt.Println("Error reading CA certificate:", err)
		return
	}

	// Create a new certificate pool and add the CA certificate
	caCertPool := x509.NewCertPool()
	caCertPool.AppendCertsFromPEM(caCert)

	// Create a new HTTPS client with the CA certificate pool
	client := &http.Client{
		Transport: &http.Transport{
			TLSClientConfig: &tls.Config{
				RootCAs: caCertPool,
			},
		},
	}

	// Make the HTTPS request
	resp, err := client.Get("https://localhost:8080")
	if err != nil {
		fmt.Println("Error making HTTPS request:", err)
		return
	}
	defer resp.Body.Close()

	// Read the response body
	body, err := io.ReadAll(resp.Body)
	if err != nil {
		fmt.Println("Error reading response body:", err)
		return
	}

	fmt.Println("Response body:", string(body))
}
