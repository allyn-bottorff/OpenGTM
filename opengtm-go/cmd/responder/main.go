// Copyright 2025 Allyn L. Bottorff
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

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
