package main

import (
	"fmt"
	hc "github.com/allyn-bottorff/OpenGTM/opengtm-go/pkg/healthcheck"
	"log"
	"net/http"
)

func main() {
	log.Println("Starting responder")
	var healthTable = hc.NewHealthTable()

	// handle table updates, PUT /table/{pool_name}
	http.HandleFunc("PUT /table/", healthTable.HandlePut)

	// handle table read
	http.HandleFunc("GET /table", healthTable.HandleGet)

	http.HandleFunc("/livez", handleLivez)
	http.HandleFunc("/readyz", handleReadyz)

	log.Fatal(http.ListenAndServe("0.0.0.0:8081", nil))

}

func handleLivez(w http.ResponseWriter, r *http.Request) {
	fmt.Fprint(w, "Healthy\n")
}

func handleReadyz(w http.ResponseWriter, r *http.Request) {
	fmt.Fprint(w, "Healthy\n")
}
